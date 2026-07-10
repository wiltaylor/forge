use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Debug, Default)]
pub struct ToggleGroupState {
    pub selected: usize,
    len: usize,
    item_rects: Vec<Rect>,
}

impl ToggleGroupState {
    pub fn new(selected: usize) -> ToggleGroupState {
        ToggleGroupState { selected, ..Default::default() }
    }

    /// Click a segment to select it.
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
        match key.code {
            KeyCode::Left => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Right => {
                if self.len > 0 && self.selected < self.len - 1 {
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

/// Segmented exclusive chip row: ` grid │ list │ table ` with the active
/// segment on a solid accent chip. ←/→ switches.
#[derive(Clone, Debug)]
pub struct ToggleGroup<'a> {
    items: &'a [&'a str],
    focused: bool,
    disabled: bool,
    theme: Option<&'a Theme>,
}

impl<'a> ToggleGroup<'a> {
    pub fn new(items: &'a [&'a str]) -> ToggleGroup<'a> {
        ToggleGroup { items, focused: false, disabled: false, theme: None }
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
}

impl<'a> StatefulWidget for ToggleGroup<'a> {
    type State = ToggleGroupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ToggleGroupState) {
        state.len = self.items.len();
        state.item_rects.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let right = area.x + area.width;
        let mut x = area.x;
        for (i, item) in self.items.iter().enumerate() {
            let w = text::width(item) as u16 + 2;
            if x + w > right {
                break;
            }
            let active = i == state.selected;
            let mut style = if self.disabled {
                Style::new().fg(t.fg[3]).bg(t.bg[2])
            } else if active {
                Style::new().fg(t.accent.contrast).bg(t.accent.base)
            } else {
                Style::new().fg(t.fg[1]).bg(t.bg[3])
            };
            if active && self.focused {
                style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            state.item_rects.push(Rect::new(x, area.y, w, 1));
            buf.set_style(Rect::new(x, area.y, w, 1), style);
            buf.set_string(x + 1, area.y, *item, style);
            x += w + 1;
        }
    }
}
