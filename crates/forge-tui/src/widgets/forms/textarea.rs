use crate::event::{is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};
use unicode_segmentation::UnicodeSegmentation;

/// Multi-line editor state. No soft wrap (like a code buffer): long lines
/// scroll horizontally, which keeps cursor math honest. `row`/`col` are the
/// cursor line index and byte offset (grapheme-aligned).
#[derive(Clone, Debug)]
pub struct TextareaState {
    lines: Vec<String>,
    row: usize,
    col: usize,
    /// Preferred display column (cells) for ↑/↓ moves.
    desired: usize,
    scroll_row: usize,
    scroll_col: usize,
    view: (u16, u16),
}

impl Default for TextareaState {
    fn default() -> TextareaState {
        TextareaState {
            lines: vec![String::new()],
            row: 0,
            col: 0,
            desired: 0,
            scroll_row: 0,
            scroll_col: 0,
            view: (0, 0),
        }
    }
}

fn cells_at(line: &str, byte: usize) -> usize {
    text::width(&line[..byte.min(line.len())])
}

fn byte_at_cells(line: &str, cells: usize) -> usize {
    let mut w = 0;
    for (i, g) in line.grapheme_indices(true) {
        let gw = text::width(g);
        if w + gw > cells {
            return i;
        }
        w += gw;
    }
    line.len()
}

impl TextareaState {
    pub fn new() -> TextareaState {
        TextareaState::default()
    }

    pub fn with_value(value: &str) -> TextareaState {
        let mut s = TextareaState::default();
        s.set_value(value);
        s
    }

    pub fn value(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_value(&mut self, value: &str) {
        self.lines = value.split('\n').map(str::to_owned).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.row = self.lines.len() - 1;
        self.col = self.lines[self.row].len();
        self.desired = cells_at(&self.lines[self.row], self.col);
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn line(&self) -> &str {
        &self.lines[self.row]
    }

    fn sync_desired(&mut self) {
        self.desired = cells_at(self.line(), self.col);
    }

    /// Insert text at the cursor (also the `Event::Paste` entry point —
    /// handles embedded newlines).
    pub fn insert_str(&mut self, s: &str) {
        for (i, part) in s.split('\n').enumerate() {
            if i > 0 {
                self.split_line();
            }
            self.lines[self.row].insert_str(self.col, part);
            self.col += part.len();
        }
        self.sync_desired();
    }

    fn split_line(&mut self) {
        let rest = self.lines[self.row].split_off(self.col);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
    }

    fn prev_boundary(&self) -> usize {
        self.line()[..self.col]
            .grapheme_indices(true)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_boundary(&self) -> usize {
        self.line()[self.col..]
            .graphemes(true)
            .next()
            .map(|g| self.col + g.len())
            .unwrap_or(self.line().len())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => {
                let mut b = [0u8; 4];
                self.insert_str(c.encode_utf8(&mut b));
                Outcome::Changed
            }
            KeyCode::Enter if ctrl => Outcome::Submitted,
            KeyCode::Enter => {
                self.split_line();
                self.sync_desired();
                Outcome::Changed
            }
            KeyCode::Backspace => {
                if self.col > 0 {
                    let to = self.prev_boundary();
                    self.lines[self.row].replace_range(to..self.col, "");
                    self.col = to;
                } else if self.row > 0 {
                    let tail = self.lines.remove(self.row);
                    self.row -= 1;
                    self.col = self.lines[self.row].len();
                    self.lines[self.row].push_str(&tail);
                } else {
                    return Outcome::Consumed;
                }
                self.sync_desired();
                Outcome::Changed
            }
            KeyCode::Delete => {
                if self.col < self.line().len() {
                    let to = self.next_boundary();
                    let from = self.col;
                    self.lines[self.row].replace_range(from..to, "");
                } else if self.row + 1 < self.lines.len() {
                    let tail = self.lines.remove(self.row + 1);
                    self.lines[self.row].push_str(&tail);
                } else {
                    return Outcome::Consumed;
                }
                Outcome::Changed
            }
            KeyCode::Left => {
                if self.col > 0 {
                    self.col = self.prev_boundary();
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.line().len();
                }
                self.sync_desired();
                Outcome::Consumed
            }
            KeyCode::Right => {
                if self.col < self.line().len() {
                    self.col = self.next_boundary();
                } else if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0;
                }
                self.sync_desired();
                Outcome::Consumed
            }
            KeyCode::Up => {
                if self.row > 0 {
                    self.row -= 1;
                    self.col = byte_at_cells(self.line(), self.desired);
                }
                Outcome::Consumed
            }
            KeyCode::Down => {
                if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = byte_at_cells(self.line(), self.desired);
                }
                Outcome::Consumed
            }
            KeyCode::Home if ctrl => {
                self.row = 0;
                self.col = 0;
                self.sync_desired();
                Outcome::Consumed
            }
            KeyCode::End if ctrl => {
                self.row = self.lines.len() - 1;
                self.col = self.line().len();
                self.sync_desired();
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.col = 0;
                self.sync_desired();
                Outcome::Consumed
            }
            KeyCode::End => {
                self.col = self.line().len();
                self.sync_desired();
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.row = self.row.saturating_sub(self.view.1.max(1) as usize);
                self.col = byte_at_cells(self.line(), self.desired);
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.row = (self.row + self.view.1.max(1) as usize).min(self.lines.len() - 1);
                self.col = byte_at_cells(self.line(), self.desired);
                Outcome::Consumed
            }
            KeyCode::Esc => Outcome::Cancelled,
            _ => Outcome::Ignored,
        }
    }
}

/// Multi-line text field, always bordered. Enter inserts a newline;
/// Ctrl+Enter submits (where the terminal can report it).
#[derive(Clone, Debug, Default)]
pub struct Textarea<'a> {
    placeholder: &'a str,
    invalid: bool,
    focused: bool,
    disabled: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Textarea<'a> {
    pub fn new() -> Textarea<'a> {
        Textarea::default()
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

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Textarea<'a> {
    type State = TextareaState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TextareaState) {
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
        let block = Block::bordered().border_style(Style::new().fg(edge));
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.is_empty() {
            return;
        }
        state.view = (inner.width, inner.height);

        // Follow the cursor.
        let cursor_cells = cells_at(&state.lines[state.row], state.col);
        if state.row < state.scroll_row {
            state.scroll_row = state.row;
        } else if state.row >= state.scroll_row + inner.height as usize {
            state.scroll_row = state.row + 1 - inner.height as usize;
        }
        if cursor_cells < state.scroll_col {
            state.scroll_col = cursor_cells;
        } else if cursor_cells >= state.scroll_col + inner.width as usize {
            state.scroll_col = cursor_cells + 1 - inner.width as usize;
        }

        let fg = if self.disabled { t.fg[3] } else { t.fg[0] };
        let empty = state.lines.len() == 1 && state.lines[0].is_empty();
        if empty && !self.placeholder.is_empty() {
            buf.set_string(
                inner.x,
                inner.y,
                text::truncate(self.placeholder, inner.width as usize),
                Style::new().fg(t.fg[3]),
            );
        } else {
            for vis in 0..inner.height as usize {
                let li = state.scroll_row + vis;
                let Some(line) = state.lines.get(li) else { break };
                let start = byte_at_cells(line, state.scroll_col);
                let visible = text::truncate(&line[start..], inner.width as usize);
                buf.set_string(inner.x, inner.y + vis as u16, visible, Style::new().fg(fg));
            }
        }

        if self.focused && !self.disabled {
            let cy = inner.y + (state.row - state.scroll_row) as u16;
            let cx = inner.x + (cursor_cells - state.scroll_col) as u16;
            if cx < inner.x + inner.width && cy < inner.y + inner.height {
                buf.set_style(
                    Rect::new(cx, cy, 1, 1),
                    Style::new().add_modifier(Modifier::REVERSED),
                );
            }
        }
    }
}
