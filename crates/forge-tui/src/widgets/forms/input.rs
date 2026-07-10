use crate::event::{is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use unicode_segmentation::UnicodeSegmentation;

/// Persistent state of an [`Input`]: value, cursor, selection, and viewport
/// scroll. The cursor is a byte offset that always sits on a grapheme
/// boundary.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    value: String,
    cursor: usize,
    anchor: Option<usize>,
    scroll: usize,
}

impl InputState {
    pub fn new() -> InputState {
        InputState::default()
    }

    pub fn with_value(value: impl Into<String>) -> InputState {
        let value = value.into();
        let cursor = value.len();
        InputState { value, cursor, anchor: None, scroll: 0 }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
        self.anchor = None;
        self.scroll = 0;
    }

    pub fn clear(&mut self) {
        self.set_value("");
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Selected byte range, if any (ordered, non-empty).
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        if a == self.cursor {
            return None;
        }
        Some((a.min(self.cursor), a.max(self.cursor)))
    }

    fn prev_boundary(&self, from: usize) -> usize {
        self.value[..from]
            .grapheme_indices(true)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_boundary(&self, from: usize) -> usize {
        self.value[from..]
            .graphemes(true)
            .next()
            .map(|g| from + g.len())
            .unwrap_or(self.value.len())
    }

    fn prev_word(&self, from: usize) -> usize {
        self.value[..from]
            .split_word_bound_indices()
            .filter(|(_, w)| !w.trim().is_empty())
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_word(&self, from: usize) -> usize {
        self.value[from..]
            .split_word_bound_indices()
            .find(|(_, w)| !w.trim().is_empty())
            .map(|(i, w)| from + i + w.len())
            .unwrap_or(self.value.len())
    }

    fn move_to(&mut self, pos: usize, select: bool) {
        if select {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
        self.cursor = pos;
    }

    fn delete_selection(&mut self) -> bool {
        if let Some((a, b)) = self.selection() {
            self.value.replace_range(a..b, "");
            self.cursor = a;
            self.anchor = None;
            true
        } else {
            self.anchor = None;
            false
        }
    }

    fn delete_range(&mut self, a: usize, b: usize) {
        if a < b {
            self.value.replace_range(a..b, "");
            self.cursor = a;
        }
        self.anchor = None;
    }

    /// Insert text at the cursor (replacing the selection) — also the paste
    /// entry point for `Event::Paste`.
    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.value.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.value.len();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let select = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Char(c) if !ctrl && !alt => {
                let mut b = [0u8; 4];
                self.insert_str(c.encode_utf8(&mut b));
                Outcome::Changed
            }
            KeyCode::Backspace if ctrl || alt => {
                let to = self.prev_word(self.cursor);
                self.delete_range(to, self.cursor);
                Outcome::Changed
            }
            KeyCode::Backspace => {
                if !self.delete_selection() {
                    if self.cursor == 0 {
                        return Outcome::Consumed;
                    }
                    let to = self.prev_boundary(self.cursor);
                    self.delete_range(to, self.cursor);
                }
                Outcome::Changed
            }
            KeyCode::Delete => {
                if !self.delete_selection() {
                    if self.cursor == self.value.len() {
                        return Outcome::Consumed;
                    }
                    let to = self.next_boundary(self.cursor);
                    let a = self.cursor;
                    self.delete_range(a, to);
                }
                Outcome::Changed
            }
            KeyCode::Left if ctrl => {
                let p = self.prev_word(self.cursor);
                self.move_to(p, select);
                Outcome::Consumed
            }
            KeyCode::Right if ctrl => {
                let p = self.next_word(self.cursor);
                self.move_to(p, select);
                Outcome::Consumed
            }
            KeyCode::Left => {
                let p = self.prev_boundary(self.cursor);
                self.move_to(p, select);
                Outcome::Consumed
            }
            KeyCode::Right => {
                let p = self.next_boundary(self.cursor);
                self.move_to(p, select);
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.move_to(0, select);
                Outcome::Consumed
            }
            KeyCode::End => {
                self.move_to(self.value.len(), select);
                Outcome::Consumed
            }
            KeyCode::Char('a') if ctrl => {
                self.move_to(0, false);
                Outcome::Consumed
            }
            KeyCode::Char('e') if ctrl => {
                self.move_to(self.value.len(), false);
                Outcome::Consumed
            }
            KeyCode::Char('b') if alt => {
                let p = self.prev_word(self.cursor);
                self.move_to(p, select);
                Outcome::Consumed
            }
            KeyCode::Char('f') if alt => {
                let p = self.next_word(self.cursor);
                self.move_to(p, select);
                Outcome::Consumed
            }
            KeyCode::Char('w') if ctrl => {
                let to = self.prev_word(self.cursor);
                self.delete_range(to, self.cursor);
                Outcome::Changed
            }
            KeyCode::Char('u') if ctrl => {
                self.delete_range(0, self.cursor);
                Outcome::Changed
            }
            KeyCode::Char('k') if ctrl => {
                let a = self.cursor;
                let b = self.value.len();
                self.delete_range(a, b);
                self.cursor = a;
                Outcome::Changed
            }
            KeyCode::Enter => Outcome::Submitted,
            KeyCode::Esc => {
                if self.selection().is_some() {
                    self.anchor = None;
                    Outcome::Consumed
                } else {
                    Outcome::Cancelled
                }
            }
            _ => Outcome::Ignored,
        }
    }
}

/// Single-line text field. One row renders as a filled strip with a state
/// bar at the left edge (accent = focused, danger = invalid); three or more
/// rows render bordered.
#[derive(Clone, Debug, Default)]
pub struct Input<'a> {
    placeholder: &'a str,
    invalid: bool,
    focused: bool,
    disabled: bool,
    masked: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Input<'a> {
    pub fn new() -> Input<'a> {
        Input::default()
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
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

    /// Render the value as `•` per grapheme (passwords).
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn render_line(&self, area: Rect, buf: &mut Buffer, t: &Theme, state: &mut InputState) {
        if area.width == 0 {
            return;
        }
        let bg = t.bg[2];
        buf.set_style(area, Style::new().bg(bg));

        let fg = if self.disabled { t.fg[3] } else { t.fg[0] };

        // Build display graphemes: (byte_offset, glyph, cell_width).
        let masked_dot = "•";
        let glyphs: Vec<(usize, &str, usize)> = state
            .value
            .grapheme_indices(true)
            .map(|(i, g)| {
                if self.masked {
                    (i, masked_dot, 1)
                } else {
                    (i, g, text::width(g))
                }
            })
            .collect();
        let cell_at = |byte: usize| -> usize {
            glyphs
                .iter()
                .take_while(|(i, _, _)| *i < byte)
                .map(|(_, _, w)| w)
                .sum()
        };

        let view_w = area.width as usize;
        // Keep the cursor inside the viewport.
        let cursor_cell = cell_at(state.cursor);
        if cursor_cell < state.scroll {
            state.scroll = cursor_cell;
        } else if cursor_cell >= state.scroll + view_w {
            state.scroll = cursor_cell + 1 - view_w;
        }

        if state.value.is_empty() {
            buf.set_string(
                area.x,
                area.y,
                text::truncate(self.placeholder, view_w),
                Style::new().fg(t.fg[3]).bg(bg),
            );
        } else {
            let selection = state.selection();
            let mut cell = 0usize;
            for (byte, g, w) in &glyphs {
                if cell + w > state.scroll + view_w {
                    break;
                }
                if cell >= state.scroll {
                    let x = area.x + (cell - state.scroll) as u16;
                    let selected = selection.is_some_and(|(a, b)| (a..b).contains(byte));
                    let style = if selected {
                        Style::new().fg(fg).bg(t.accent.bg)
                    } else {
                        Style::new().fg(fg).bg(bg)
                    };
                    buf.set_string(x, area.y, *g, style);
                }
                cell += w;
            }
        }

        // Cursor block (rendered, not the hardware cursor, so it snapshots).
        if self.focused && !self.disabled && cursor_cell >= state.scroll {
            let x = area.x + (cursor_cell - state.scroll) as u16;
            if x < area.x + area.width {
                buf.set_style(
                    Rect::new(x, area.y, 1, 1),
                    Style::new().add_modifier(Modifier::REVERSED),
                );
            }
        }
    }
}

impl<'a> StatefulWidget for Input<'a> {
    type State = InputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut InputState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let edge = if self.invalid {
            t.danger.base
        } else if self.focused {
            t.accent.base
        } else {
            t.border.default
        };
        if area.height >= 3 {
            let block = Block::bordered().border_style(Style::new().fg(edge));
            let inner = block.inner(area);
            block.render(area, buf);
            let line = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1);
            self.render_line(line, buf, t, state);
        } else {
            buf.set_string(area.x, area.y, "▎", Style::new().fg(edge).bg(t.bg[2]));
            let line = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1);
            self.render_line(line, buf, t, state);
        }
    }
}
