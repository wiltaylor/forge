use crate::event::{is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug, Default)]
pub struct CheckboxState {
    pub checked: bool,
}

impl CheckboxState {
    pub fn new(checked: bool) -> CheckboxState {
        CheckboxState { checked }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.checked = !self.checked;
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// `[✓] label` — Space/Enter toggles (via [`CheckboxState::handle_key`]).
#[derive(Clone, Debug)]
pub struct Checkbox<'a> {
    label: &'a str,
    focused: bool,
    disabled: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Checkbox<'a> {
    pub fn new(label: &'a str) -> Checkbox<'a> {
        Checkbox { label, focused: false, disabled: false, theme: None }
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

impl<'a> StatefulWidget for Checkbox<'a> {
    type State = CheckboxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut CheckboxState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let bracket = Style::new().fg(if self.disabled {
            t.fg[3]
        } else if self.focused {
            t.accent.base
        } else {
            t.fg[2]
        });
        let mark_color = if self.disabled { t.fg[3] } else { t.accent.base };
        buf.set_string(area.x, area.y, "[", bracket);
        buf.set_string(
            area.x + 1,
            area.y,
            if state.checked { "✓" } else { " " },
            Style::new().fg(mark_color),
        );
        buf.set_string(area.x + 2, area.y, "]", bracket);
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
