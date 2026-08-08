use crate::text;
use crate::theme::TextRole;
use crate::widgets::hit::ToggleState;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

/// On/off switch: `○──` (off) / `──●` (on, accent). Space/Enter toggles.
#[derive(Clone, Debug)]
pub struct Toggle<'a> {
    label: &'a str,
    focused: bool,
    disabled: bool,
}

impl<'a> Toggle<'a> {
    pub fn new(label: &'a str) -> Toggle<'a> {
        Toggle {
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

impl<'a> StatefulWidget for Toggle<'a> {
    type State = ToggleState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ToggleState) {
        state.set_area(Rect::new(area.x, area.y, area.width, 1));
        paint(area, |t| {
            let (track, knob) = if self.disabled {
                (t.text(TextRole::Disabled), t.text(TextRole::Disabled))
            } else if state.on {
                (t.accent.base, t.accent.base)
            } else {
                (t.border.strong, t.text(TextRole::Tertiary))
            };
            let switch = if state.on { "──●" } else { "○──" };
            // Paint track and knob separately so the knob pops.
            buf.set_string(area.x, area.y, switch, Style::new().fg(track));
            let knob_x = if state.on { area.x + 2 } else { area.x };
            buf.set_string(
                knob_x,
                area.y,
                if state.on { "●" } else { "○" },
                Style::new().fg(knob),
            );
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
