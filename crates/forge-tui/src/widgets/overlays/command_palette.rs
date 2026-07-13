use crate::event::{in_area, is_press, left_down, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use crate::widgets::forms::{Input, InputState};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, StatefulWidget, Widget};

/// A palette command.
#[derive(Clone, Copy, Debug)]
pub struct Command<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub kbd: Option<&'a str>,
}

impl<'a> Command<'a> {
    pub fn new(id: &'a str, label: &'a str) -> Command<'a> {
        Command {
            id,
            label,
            kbd: None,
        }
    }

    pub fn kbd(mut self, kbd: &'a str) -> Self {
        self.kbd = Some(kbd);
        self
    }
}

/// Fuzzy command-palette state (Ctrl+K). Type to rank, ↑/↓ to move, Enter
/// submits — read [`PaletteState::highlighted`] for the chosen index.
#[derive(Clone, Debug, Default)]
pub struct PaletteState {
    pub input: InputState,
    highlight: usize,
    filtered: Vec<usize>,
    offset: usize,
    view_h: usize,
    panel: Rect,
    list_area: Rect,
}

impl PaletteState {
    pub fn new() -> PaletteState {
        PaletteState::default()
    }

    /// Index into the command slice of the highlighted match.
    pub fn highlighted(&self) -> Option<usize> {
        self.filtered.get(self.highlight).copied()
    }

    pub fn matches(&self) -> &[usize] {
        &self.filtered
    }

    pub fn filter(&mut self, commands: &[Command]) {
        let needle = self.input.value();
        let mut scored: Vec<(i64, usize)> = commands
            .iter()
            .enumerate()
            .filter_map(|(i, c)| text::fuzzy_score(needle, c.label).map(|s| (s, i)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        self.highlight = self.highlight.min(self.filtered.len().saturating_sub(1));
    }

    /// Hover highlights, click runs the command under the pointer, wheel
    /// scrolls, click-away cancels.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta != 0 && in_area(ev, self.panel) {
            self.highlight = if delta < 0 {
                self.highlight.saturating_sub(1)
            } else {
                (self.highlight + 1).min(self.filtered.len().saturating_sub(1))
            };
            return Outcome::Consumed;
        }
        if matches!(ev.kind, MouseEventKind::Moved) && in_area(ev, self.list_area) {
            let fi = self.offset + (ev.row - self.list_area.y) as usize;
            if fi < self.filtered.len() {
                self.highlight = fi;
            }
            return Outcome::Consumed;
        }
        if !left_down(ev) {
            return Outcome::Ignored;
        }
        if !in_area(ev, self.panel) {
            return Outcome::Cancelled; // click-away
        }
        if in_area(ev, self.list_area) {
            let fi = self.offset + (ev.row - self.list_area.y) as usize;
            if fi < self.filtered.len() {
                self.highlight = fi;
                return Outcome::Submitted;
            }
        }
        Outcome::Consumed
    }

    pub fn handle_key(&mut self, key: KeyEvent, commands: &[Command]) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Down => {
                if self.highlight + 1 < self.filtered.len() {
                    self.highlight += 1;
                }
                Outcome::Consumed
            }
            KeyCode::Up => {
                self.highlight = self.highlight.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Enter => {
                if self.highlighted().is_some() {
                    Outcome::Submitted
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Esc => Outcome::Cancelled,
            _ => {
                let out = self.input.handle_key(key);
                if out == Outcome::Changed {
                    self.highlight = 0;
                    self.filter(commands);
                }
                if out.is_handled() {
                    Outcome::Consumed
                } else {
                    Outcome::Ignored
                }
            }
        }
    }
}

/// The palette panel: top-centered, an input row over the ranked matches.
#[derive(Clone, Debug)]
pub struct Palette<'a> {
    commands: &'a [Command<'a>],
    max_rows: u16,
    theme: Option<&'a Theme>,
}

impl<'a> Palette<'a> {
    pub fn new(commands: &'a [Command<'a>]) -> Palette<'a> {
        Palette {
            commands,
            max_rows: 10,
            theme: None,
        }
    }

    pub fn max_rows(mut self, rows: u16) -> Self {
        self.max_rows = rows;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Palette<'a> {
    type State = PaletteState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut PaletteState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        // First render with an untouched filter: rank everything.
        if state.filtered.is_empty() && state.input.value().is_empty() {
            state.filter(self.commands);
        }
        let rows = (state.filtered.len() as u16).clamp(1, self.max_rows);
        let w = area.width.saturating_sub(4).min(64).max(20);
        let h = (rows + 4).min(area.height);
        let panel = Rect::new(area.x + (area.width - w) / 2, area.y + 1, w, h);
        state.panel = panel;
        Clear.render(panel, buf);
        let block = Block::bordered()
            .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
            .style(Style::new().bg(t.bg[4]));
        let inner = block.inner(panel);
        block.render(panel, buf);
        if inner.height < 2 {
            return;
        }
        Input::new()
            .placeholder("Type a command…")
            .focused(true)
            .theme(t)
            .render(
                Rect::new(inner.x, inner.y, inner.width, 1),
                buf,
                &mut state.input,
            );

        let list = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
        state.view_h = list.height as usize;
        state.list_area = list;
        if state.highlight < state.offset {
            state.offset = state.highlight;
        } else if state.highlight >= state.offset + state.view_h {
            state.offset = state.highlight + 1 - state.view_h;
        }
        if state.filtered.is_empty() {
            buf.set_string(
                list.x + 1,
                list.y,
                "No matching commands",
                Style::new().fg(t.fg[3]).bg(t.bg[4]),
            );
            return;
        }
        for vis in 0..state.view_h {
            let fi = state.offset + vis;
            let Some(&ci) = state.filtered.get(fi) else {
                break;
            };
            let cmd = &self.commands[ci];
            let y = list.y + vis as u16;
            let is_cursor = fi == state.highlight;
            let mut style = Style::new().fg(t.fg[1]).bg(t.bg[4]);
            if is_cursor {
                style = Style::new()
                    .fg(t.fg[0])
                    .bg(t.bg[3])
                    .add_modifier(Modifier::BOLD);
                buf.set_style(Rect::new(list.x, y, list.width, 1), style);
            }
            buf.set_string(
                list.x + 1,
                y,
                text::truncate(cmd.label, list.width.saturating_sub(2) as usize),
                style,
            );
            if let Some(kbd) = cmd.kbd {
                let kw = text::width(kbd) as u16;
                if list.width > kw + 2 {
                    buf.set_string(
                        list.x + list.width - kw - 1,
                        y,
                        kbd,
                        Style::new()
                            .fg(t.fg[2])
                            .bg(if is_cursor { t.bg[3] } else { t.bg[4] }),
                    );
                }
            }
        }
    }
}
