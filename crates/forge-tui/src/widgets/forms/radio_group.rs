use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

/// Selection state for a [`RadioGroup`]. `len` is captured at render time so
/// navigation can clamp — handle-before-first-render simply doesn't move.
#[derive(Clone, Debug, Default)]
pub struct RadioState {
    pub selected: usize,
    len: usize,
    item_rects: Vec<Rect>,
}

impl RadioState {
    pub fn new(selected: usize) -> RadioState {
        RadioState { selected, ..Default::default() }
    }

    /// Click an option to select it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        for (i, rect) in self.item_rects.iter().enumerate() {
            if clicked(ev, *rect) {
                let changed = self.selected != i;
                self.selected = i;
                return if changed { Outcome::Changed } else { Outcome::Consumed };
            }
        }
        Outcome::Ignored
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let last = self.len.saturating_sub(1);
        match key.code {
            KeyCode::Up | KeyCode::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Down | KeyCode::Right => {
                if self.len > 0 && self.selected < last {
                    self.selected += 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            _ => Outcome::Ignored,
        }
    }
}

/// Exclusive choice list: `(•) option`. Vertical by default; horizontal packs
/// options on one row.
#[derive(Clone, Debug)]
pub struct RadioGroup<'a> {
    items: &'a [&'a str],
    horizontal: bool,
    focused: bool,
    disabled: bool,
    theme: Option<&'a Theme>,
}

impl<'a> RadioGroup<'a> {
    pub fn new(items: &'a [&'a str]) -> RadioGroup<'a> {
        RadioGroup {
            items,
            horizontal: false,
            focused: false,
            disabled: false,
            theme: None,
        }
    }

    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn item_style(&self, t: &Theme, selected: bool) -> (Style, Style) {
        let mark = if self.disabled {
            Style::new().fg(t.fg[3])
        } else if selected {
            Style::new().fg(t.accent.base)
        } else {
            Style::new().fg(t.fg[2])
        };
        let mut label = if self.disabled {
            Style::new().fg(t.fg[3])
        } else if selected {
            Style::new().fg(t.fg[0])
        } else {
            Style::new().fg(t.fg[1])
        };
        if self.focused && selected {
            label = label.add_modifier(Modifier::UNDERLINED);
        }
        (mark, label)
    }
}

impl<'a> StatefulWidget for RadioGroup<'a> {
    type State = RadioState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut RadioState) {
        state.len = self.items.len();
        state.item_rects.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        if self.horizontal {
            let mut x = area.x;
            for (i, item) in self.items.iter().enumerate() {
                let selected = i == state.selected;
                let (mark, label) = self.item_style(t, selected);
                let cell = format!("({})", if selected { "•" } else { " " });
                let need = 4 + text::width(item) as u16 + 2;
                if x + need > area.x + area.width {
                    break;
                }
                state.item_rects.push(Rect::new(x, area.y, need, 1));
                buf.set_string(x, area.y, cell, mark);
                buf.set_string(x + 4, area.y, *item, label);
                x += need;
            }
        } else {
            for (i, item) in self.items.iter().enumerate() {
                if i as u16 >= area.height {
                    break;
                }
                let selected = i == state.selected;
                let (mark, label) = self.item_style(t, selected);
                let y = area.y + i as u16;
                state.item_rects.push(Rect::new(area.x, y, area.width, 1));
                buf.set_string(area.x, y, format!("({})", if selected { "•" } else { " " }), mark);
                if area.width > 4 {
                    buf.set_string(
                        area.x + 4,
                        y,
                        text::truncate(item, area.width as usize - 4),
                        label,
                    );
                }
            }
        }
    }
}
