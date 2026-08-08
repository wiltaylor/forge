use crate::event::{is_press, Outcome};
use crate::text;
use crate::theme::{TextRole, Theme};
use crate::widgets::hit::RectCache;
use crate::widgets::paint;
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
    item_rects: RectCache,
}

impl RadioState {
    pub fn new(selected: usize) -> RadioState {
        RadioState {
            selected,
            ..Default::default()
        }
    }

    /// Click an option to select it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        self.item_rects.select(ev, &mut self.selected)
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
}

impl<'a> RadioGroup<'a> {
    pub fn new(items: &'a [&'a str]) -> RadioGroup<'a> {
        RadioGroup {
            items,
            horizontal: false,
            focused: false,
            disabled: false,
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

    fn item_style(&self, t: &Theme, selected: bool) -> (Style, Style) {
        let mark = if self.disabled {
            Style::new().fg(t.text(TextRole::Disabled))
        } else if selected {
            Style::new().fg(t.accent.base)
        } else {
            Style::new().fg(t.text(TextRole::Tertiary))
        };
        let mut label = if self.disabled {
            Style::new().fg(t.text(TextRole::Disabled))
        } else if selected {
            Style::new().fg(t.text(TextRole::Primary))
        } else {
            Style::new().fg(t.text(TextRole::Secondary))
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
        paint(area, |t| {
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
                    buf.set_string(
                        area.x,
                        y,
                        format!("({})", if selected { "•" } else { " " }),
                        mark,
                    );
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
        });
    }
}
