use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Debug, Default)]
pub struct TabsState {
    pub selected: usize,
    len: usize,
    label_rects: Vec<Rect>,
}

impl TabsState {
    pub fn new(selected: usize) -> TabsState {
        TabsState { selected, ..Default::default() }
    }

    /// Click a tab label to select it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        for (i, rect) in self.label_rects.iter().enumerate() {
            if clicked(ev, *rect) {
                let changed = self.selected != i;
                self.selected = i;
                return if changed { Outcome::Changed } else { Outcome::Consumed };
            }
        }
        Outcome::Ignored
    }

    /// ←/→ (or [ / ]) switch tabs; 1–9 jump directly.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let last = self.len.saturating_sub(1);
        match key.code {
            KeyCode::Left | KeyCode::Char('[') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Right | KeyCode::Char(']') => {
                if self.len > 0 && self.selected < last {
                    self.selected += 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Char(c @ '1'..='9') => {
                let idx = c as usize - '1' as usize;
                if idx < self.len && idx != self.selected {
                    self.selected = idx;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            _ => Outcome::Ignored,
        }
    }
}

/// Underline-style tab row. Two rows when it gets them: labels + an accent
/// underline under the active tab; one row falls back to bold/accent text.
#[derive(Clone, Debug)]
pub struct Tabs<'a> {
    labels: &'a [&'a str],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Tabs<'a> {
    pub fn new(labels: &'a [&'a str]) -> Tabs<'a> {
        Tabs { labels, focused: false, theme: None }
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

impl<'a> StatefulWidget for Tabs<'a> {
    type State = TabsState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TabsState) {
        state.len = self.labels.len();
        state.label_rects.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let two_rows = area.height >= 2;
        if two_rows {
            buf.set_string(
                area.x,
                area.y + 1,
                "─".repeat(area.width as usize),
                Style::new().fg(t.border.subtle),
            );
        }
        let right = area.x + area.width;
        let mut x = area.x;
        for (i, label) in self.labels.iter().enumerate() {
            let lw = text::width(label) as u16;
            if x + lw > right {
                break;
            }
            state.label_rects.push(Rect::new(x, area.y, lw, 1));
            let active = i == state.selected;
            let mut style = Style::new().fg(if active { t.fg[0] } else { t.fg[1] });
            if active {
                style = style.add_modifier(Modifier::BOLD);
                if !two_rows {
                    style = style.fg(t.accent.fg).add_modifier(Modifier::UNDERLINED);
                }
                if self.focused {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
            }
            buf.set_string(x, area.y, *label, style);
            if active && two_rows {
                buf.set_string(
                    x,
                    area.y + 1,
                    "━".repeat(lw as usize),
                    Style::new().fg(t.accent.base),
                );
            }
            x += lw + 3;
        }
    }
}
