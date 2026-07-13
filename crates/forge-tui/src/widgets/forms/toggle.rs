use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug, Default)]
pub struct ToggleState {
    area: Rect,
    pub on: bool,
}

impl ToggleState {
    pub fn new(on: bool) -> ToggleState {
        ToggleState {
            on,
            area: Rect::default(),
        }
    }

    /// Click anywhere on the control toggles it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if clicked(ev, self.area) {
            self.on = !self.on;
            Outcome::Changed
        } else {
            Outcome::Ignored
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.on = !self.on;
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// On/off switch: `○──` (off) / `──●` (on, accent). Space/Enter toggles.
#[derive(Clone, Debug)]
pub struct Toggle<'a> {
    label: &'a str,
    focused: bool,
    disabled: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Toggle<'a> {
    pub fn new(label: &'a str) -> Toggle<'a> {
        Toggle {
            label,
            focused: false,
            disabled: false,
            theme: None,
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

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Toggle<'a> {
    type State = ToggleState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ToggleState) {
        state.area = Rect::new(area.x, area.y, area.width, 1);
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let (track, knob) = if self.disabled {
            (t.fg[3], t.fg[3])
        } else if state.on {
            (t.accent.base, t.accent.base)
        } else {
            (t.border.strong, t.fg[2])
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
            let mut style = Style::new().fg(if self.disabled { t.fg[3] } else { t.fg[0] });
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
    }
}
