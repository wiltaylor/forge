use crate::event::{clicked, is_press, left_down, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use crate::widgets::forms::{ListBox, ListBoxState};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Clear, StatefulWidget, Widget};

/// Closed field + dropdown list. `value` is the committed choice; the list
/// cursor only becomes the value on Enter.
#[derive(Clone, Debug, Default)]
pub struct SelectState {
    pub open: bool,
    pub value: Option<usize>,
    pub list: ListBoxState,
    field: Rect,
}

impl SelectState {
    pub fn new() -> SelectState {
        SelectState::default()
    }

    pub fn with_value(value: usize) -> SelectState {
        SelectState { value: Some(value), ..Default::default() }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        if !self.open {
            return match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Down => {
                    self.open = true;
                    self.list.highlight = self.value.unwrap_or(0);
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            };
        }
        match key.code {
            KeyCode::Esc => {
                self.open = false;
                Outcome::Consumed
            }
            KeyCode::Enter => {
                self.value = Some(self.list.highlight);
                self.open = false;
                Outcome::Changed
            }
            _ => match self.list.handle_key(key) {
                Outcome::Ignored => Outcome::Consumed, // trap keys while open
                o => o,
            },
        }
    }

    /// Type-ahead within the dropdown.
    pub fn jump_to(&mut self, items: &[&str], c: char) -> Outcome {
        self.list.jump_to(items, c)
    }

    /// Click the field to open/close; click an option to commit it; click
    /// away to dismiss; wheel moves the dropdown cursor.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if !self.open {
            if clicked(ev, self.field) {
                self.open = true;
                self.list.highlight = self.value.unwrap_or(0);
                return Outcome::Consumed;
            }
            return Outcome::Ignored;
        }
        if let Some(row) = self.list.row_at(ev).filter(|_| left_down(ev)) {
            self.value = Some(row);
            self.open = false;
            return Outcome::Changed;
        }
        match self.list.handle_mouse(ev) {
            Outcome::Ignored if left_down(ev) => {
                // Click-away closes without committing.
                self.open = false;
                if clicked(ev, self.field) {
                    return Outcome::Consumed;
                }
                Outcome::Consumed
            }
            Outcome::Ignored => Outcome::Ignored,
            _ => Outcome::Consumed,
        }
    }
}

/// The Forge select. Give it one row; when open it overdraws a popup panel
/// on the rows below — render it after (or above) the content beneath, as
/// with any floating element.
#[derive(Clone, Debug)]
pub struct Select<'a> {
    items: &'a [&'a str],
    placeholder: &'a str,
    focused: bool,
    disabled: bool,
    max_popup: u16,
    theme: Option<&'a Theme>,
}

impl<'a> Select<'a> {
    pub fn new(items: &'a [&'a str]) -> Select<'a> {
        Select {
            items,
            placeholder: "Select…",
            focused: false,
            disabled: false,
            max_popup: 8,
            theme: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
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

    pub fn max_popup(mut self, rows: u16) -> Self {
        self.max_popup = rows;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Select<'a> {
    type State = SelectState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut SelectState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let edge = if self.focused && !self.disabled {
            t.accent.base
        } else {
            t.border.default
        };
        let field = Rect::new(area.x, area.y, area.width, 1);
        state.field = field;
        buf.set_style(field, Style::new().bg(t.bg[2]));
        buf.set_string(area.x, area.y, "▎", Style::new().fg(edge).bg(t.bg[2]));
        let (label, style) = match state.value.and_then(|i| self.items.get(i)) {
            Some(v) => (*v, Style::new().fg(if self.disabled { t.fg[3] } else { t.fg[0] })),
            None => (self.placeholder, Style::new().fg(t.fg[3])),
        };
        buf.set_string(
            area.x + 1,
            area.y,
            text::truncate(label, area.width.saturating_sub(4) as usize),
            style.bg(t.bg[2]),
        );
        if area.width >= 3 {
            buf.set_string(
                area.x + area.width - 2,
                area.y,
                if state.open { "▴" } else { "▾" },
                Style::new().fg(t.fg[2]).bg(t.bg[2]),
            );
        }

        if state.open && !self.disabled {
            let rows = (self.items.len() as u16).min(self.max_popup);
            let popup = Rect::new(area.x, area.y + 1, area.width, rows + 2)
                .intersection(buf.area);
            if popup.height >= 3 {
                Clear.render(popup, buf);
                let block = Block::bordered()
                    .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
                    .style(Style::new().bg(t.bg[4]));
                let inner = block.inner(popup);
                block.render(popup, buf);
                // The committed value is shown as a display-only selection;
                // the persistent list state keeps highlight/scroll.
                let mut display = state.list.clone();
                match state.value {
                    Some(v) => display.select_only(v),
                    None => display.clear_selection(),
                }
                ListBox::new(self.items).focused(self.focused).theme(t).render(
                    inner,
                    buf,
                    &mut display,
                );
                display.clear_selection();
                state.list = display;
            }
        }
    }
}
