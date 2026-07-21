use crate::event::{in_area, is_press, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Trace => "TRC",
            Level::Debug => "DBG",
            Level::Info => "INF",
            Level::Warn => "WRN",
            Level::Error => "ERR",
        }
    }

    fn color(self, t: &Theme) -> Color {
        match self {
            Level::Trace => t.fg[3],
            Level::Debug => t.fg[2],
            Level::Info => t.info.base,
            Level::Warn => t.warning.base,
            Level::Error => t.danger.base,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogLine {
    pub level: Level,
    pub ts: Option<String>,
    pub text: String,
}

impl LogLine {
    pub fn new(level: Level, text: impl Into<String>) -> LogLine {
        LogLine {
            level,
            ts: None,
            text: text.into(),
        }
    }

    pub fn ts(mut self, ts: impl Into<String>) -> LogLine {
        self.ts = Some(ts.into());
        self
    }
}

/// Follow-mode scrollback. `follow` pins the view to the tail; any upward
/// scroll unpins, End/`f` re-pins. Set `search` to highlight matches.
#[derive(Clone, Debug)]
pub struct LogsState {
    pub follow: bool,
    pub search: Option<String>,
    offset: usize,
    len: usize,
    view_h: usize,
    area: Rect,
}

impl Default for LogsState {
    fn default() -> LogsState {
        LogsState {
            follow: true,
            search: None,
            offset: 0,
            len: 0,
            view_h: 0,
            area: Rect::default(),
        }
    }
}

impl LogsState {
    pub fn new() -> LogsState {
        LogsState::default()
    }

    /// Wheel scrolls (scrolling up unpins follow mode).
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta == 0 || !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        if delta < 0 {
            self.follow = false;
            self.offset = self.offset.saturating_sub(3);
        } else {
            self.offset = (self.offset + 3).min(self.len.saturating_sub(self.view_h));
        }
        Outcome::Consumed
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let page = self.view_h.max(1);
        match key.code {
            KeyCode::Up => {
                self.follow = false;
                self.offset = self.offset.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.offset = (self.offset + 1).min(self.len.saturating_sub(self.view_h));
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.follow = false;
                self.offset = self.offset.saturating_sub(page);
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.offset = (self.offset + page).min(self.len.saturating_sub(self.view_h));
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.follow = false;
                self.offset = 0;
                Outcome::Consumed
            }
            KeyCode::End | KeyCode::Char('f') => {
                self.follow = true;
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// Level-colored monospace log stream. No soft wrap — long lines truncate
/// with an ellipsis (pipe through a pager for full text).
#[derive(Clone, Debug)]
pub struct Logs<'a> {
    lines: &'a [LogLine],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Logs<'a> {
    pub fn new(lines: &'a [LogLine]) -> Logs<'a> {
        Logs {
            lines,
            focused: false,
            theme: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Logs<'a> {
    type State = LogsState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut LogsState) {
        state.len = self.lines.len();
        state.view_h = area.height as usize;
        state.area = area;
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        buf.set_style(area, Style::new().bg(t.bg[1]));
        let max_offset = state.len.saturating_sub(state.view_h);
        if state.follow {
            state.offset = max_offset;
        } else {
            state.offset = state.offset.min(max_offset);
        }
        for vis in 0..state.view_h {
            let li = state.offset + vis;
            let Some(line) = self.lines.get(li) else {
                break;
            };
            let y = area.y + vis as u16;
            let mut x = area.x;
            if let Some(ts) = &line.ts {
                let tw = text::width(ts) as u16;
                if area.width > tw + 6 {
                    buf.set_string(x, y, ts, Style::new().fg(t.fg[3]).bg(t.bg[1]));
                    x += tw + 1;
                }
            }
            buf.set_string(
                x,
                y,
                line.level.label(),
                Style::new()
                    .fg(line.level.color(t))
                    .bg(t.bg[1])
                    .add_modifier(Modifier::BOLD),
            );
            x += 4;
            let avail = (area.x + area.width).saturating_sub(x) as usize;
            let shown = text::truncate(&line.text, avail);
            buf.set_string(x, y, &shown, Style::new().fg(t.fg[1]).bg(t.bg[1]));
            // Search highlight (case-insensitive substring on the visible slice).
            if let Some(needle) = state.search.as_deref().filter(|n| !n.is_empty()) {
                let hay = shown.to_lowercase();
                let needle_l = needle.to_lowercase();
                // NB: indices come from the lowercased haystack; Unicode
                // lowercasing can shift byte offsets, so use checked slicing
                // and skip highlights that land off a char boundary.
                let mut start = 0;
                while let Some(pos) = hay.get(start..).and_then(|h| h.find(&needle_l)) {
                    let at = start + pos;
                    start = at + needle_l.len().max(1);
                    let (Some(prefix), Some(matched)) =
                        (shown.get(..at), shown.get(at..at + needle_l.len()))
                    else {
                        continue;
                    };
                    let prefix_cells = text::width(prefix) as u16;
                    let match_cells = text::width(matched) as u16;
                    buf.set_style(
                        Rect::new(x + prefix_cells, y, match_cells, 1),
                        Style::new().fg(t.accent.fg).bg(t.accent.bg),
                    );
                }
            }
        }
        // Follow indicator.
        if state.follow && self.focused && area.height > 0 {
            let tag = " follow ";
            let wx = area.x + area.width.saturating_sub(tag.len() as u16 + 1);
            buf.set_string(
                wx,
                area.y,
                tag,
                Style::new().fg(t.accent.fg).bg(t.accent.bg),
            );
        }
    }
}
