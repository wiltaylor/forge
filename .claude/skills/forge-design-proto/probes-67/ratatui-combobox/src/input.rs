//! `input` — ratatui.
//!
//! GAP. `reference/controls/input.md` and `reference/impl/ratatui/input.md` both say
//! "Not written", and `reference/gaps.md` carries no row for the pair. Built from
//! `reference/laws.md` and `reference/ratatui.md` only, and reported as such.
//!
//! `combobox` composes this control rather than reimplementing text editing.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::StatefulWidget;

use crate::outcome::Outcome;
use crate::theme::Theme;

/// A single-line text field. The app owns it; the widget is rebuilt each frame.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    chars: Vec<char>,
    /// Caret position, counted in characters.
    cursor: usize,
    /// The whole value is selected, so the next character typed replaces it.
    ///
    /// A terminal has no selection model, so "select all" is this flag plus a reversed
    /// run at draw time.
    selected_all: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The current text.
    pub fn value(&self) -> String {
        self.chars.iter().collect()
    }

    /// True when the field holds nothing.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Replace the text and put the caret at the end.
    pub fn set_value(&mut self, value: impl AsRef<str>) {
        self.chars = value.as_ref().chars().collect();
        self.cursor = self.chars.len();
        self.selected_all = false;
    }

    /// Empty the field.
    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
        self.selected_all = false;
    }

    /// Caret position in characters.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Select the whole value, so the next character typed replaces it.
    pub fn select_all(&mut self) {
        self.selected_all = !self.chars.is_empty();
        self.cursor = self.chars.len();
    }

    /// True while the whole value is selected.
    pub fn is_all_selected(&self) -> bool {
        self.selected_all
    }

    /// Route one key. The caller decides this control has focus.
    ///
    /// Reacts to key **presses** only: Windows reports press and release, and reacting to
    /// release double-triggers every control.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match key.code {
            KeyCode::Char(c) if plain => {
                if self.selected_all {
                    self.chars.clear();
                    self.cursor = 0;
                    self.selected_all = false;
                }
                self.chars.insert(self.cursor, c);
                self.cursor += 1;
                Outcome::Changed
            }
            KeyCode::Backspace => {
                if self.selected_all {
                    self.clear();
                    return Outcome::Changed;
                }
                if self.cursor == 0 {
                    return Outcome::Consumed;
                }
                self.cursor -= 1;
                self.chars.remove(self.cursor);
                Outcome::Changed
            }
            KeyCode::Delete => {
                if self.selected_all {
                    self.clear();
                    return Outcome::Changed;
                }
                if self.cursor >= self.chars.len() {
                    return Outcome::Consumed;
                }
                self.chars.remove(self.cursor);
                Outcome::Changed
            }
            KeyCode::Left => {
                self.selected_all = false;
                self.cursor = self.cursor.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Right => {
                self.selected_all = false;
                self.cursor = (self.cursor + 1).min(self.chars.len());
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.selected_all = false;
                self.cursor = 0;
                Outcome::Consumed
            }
            KeyCode::End => {
                self.selected_all = false;
                self.cursor = self.chars.len();
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }

    /// First visible character, so a value longer than the field scrolls with the caret.
    fn scroll_offset(&self, width: usize) -> usize {
        if width == 0 || self.cursor < width {
            0
        } else {
            self.cursor + 1 - width
        }
    }
}

/// The `input` widget. One cell row — `laws.md` sizes a control at one cell in a terminal.
pub struct Input<'a> {
    theme: &'a Theme,
    focused: bool,
    /// Shown when the field is empty. Every visible string is a parameter.
    placeholder: &'a str,
}

impl<'a> Input<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            focused: false,
            placeholder: "",
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }
}

impl StatefulWidget for Input<'_> {
    type State = InputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut InputState) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let row = Rect {
            height: 1,
            ..area
        };
        let theme = self.theme;
        let width = row.width as usize;

        if state.chars.is_empty() {
            let text: String = self.placeholder.chars().take(width).collect();
            buf.set_string(row.x, row.y, text, Style::default().fg(theme.fg[3]));
        } else {
            let offset = state.scroll_offset(width);
            let text: String = state.chars.iter().skip(offset).take(width).collect();
            let style = if state.selected_all {
                // No selection model in a terminal: a reversed run is the selection.
                Style::default().fg(theme.fg[0]).reversed()
            } else {
                Style::default().fg(theme.fg[0])
            };
            buf.set_string(row.x, row.y, text, style);
        }

        if self.focused && !state.selected_all {
            let offset = state.scroll_offset(width);
            let caret = state.cursor.saturating_sub(offset);
            if caret < width {
                let x = row.x + caret as u16;
                buf.set_style(
                    Rect {
                        x,
                        y: row.y,
                        width: 1,
                        height: 1,
                    },
                    Style::default().reversed(),
                );
            }
        }
    }
}
