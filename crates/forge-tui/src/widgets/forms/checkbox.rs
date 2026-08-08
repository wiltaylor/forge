use crate::text;
use crate::theme::TextRole;
use crate::widgets::hit::ToggleState;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

/// A checkbox is the shared click-to-toggle state; `on` means checked.
pub type CheckboxState = ToggleState;

/// `[✓] label` — Space/Enter toggles (via [`ToggleState::handle_key`]).
#[derive(Clone, Debug)]
pub struct Checkbox<'a> {
    label: &'a str,
    focused: bool,
    disabled: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(label: &'a str) -> Checkbox<'a> {
        Checkbox {
            label,
            focused: false,
            disabled: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a> StatefulWidget for Checkbox<'a> {
    type State = CheckboxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut CheckboxState) {
        state.set_area(Rect::new(area.x, area.y, area.width, 1));
        paint(area, |t| {
            let bracket = Style::new().fg(if self.disabled {
                t.text(TextRole::Disabled)
            } else if self.focused {
                t.accent.base
            } else {
                t.text(TextRole::Tertiary)
            });
            let mark_color = if self.disabled {
                t.text(TextRole::Disabled)
            } else {
                t.accent.base
            };
            buf.set_string(area.x, area.y, "[", bracket);
            buf.set_string(
                area.x + 1,
                area.y,
                if state.on { "✓" } else { " " },
                Style::new().fg(mark_color),
            );
            buf.set_string(area.x + 2, area.y, "]", bracket);
            if area.width > 4 {
                let mut style = Style::new().fg(if self.disabled {
                    t.text(TextRole::Disabled)
                } else {
                    t.text(TextRole::Primary)
                });
                if self.focused {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                buf.set_string(
                    area.x + 4,
                    area.y,
                    text::truncate(self.label, area.width as usize - 4),
                    style,
                );
            }
        });
    }
}
