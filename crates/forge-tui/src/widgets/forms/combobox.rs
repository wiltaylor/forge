use crate::event::{is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use crate::widgets::forms::{Input, InputState};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, StatefulWidget, Widget};

/// Input + fuzzy-filtered dropdown. Filtering uses the zero-dep subsequence
/// scorer in [`crate::text::fuzzy_score`]; ranking happens per keystroke via
/// [`ComboboxState::filter`], which the state runs automatically on change.
#[derive(Clone, Debug, Default)]
pub struct ComboboxState {
    pub input: InputState,
    pub open: bool,
    highlight: usize,
    filtered: Vec<usize>,
    view_h: usize,
    offset: usize,
}

impl ComboboxState {
    pub fn new() -> ComboboxState {
        ComboboxState::default()
    }

    /// Indices (into the item slice) of the current matches, best first.
    pub fn matches(&self) -> &[usize] {
        &self.filtered
    }

    /// The highlighted match, as an index into the item slice.
    pub fn highlighted(&self) -> Option<usize> {
        self.filtered.get(self.highlight).copied()
    }

    /// Re-rank items against the current input value.
    pub fn filter(&mut self, items: &[&str]) {
        let needle = self.input.value();
        let mut scored: Vec<(i64, usize)> = items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| text::fuzzy_score(needle, item).map(|s| (s, i)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        self.highlight = self.highlight.min(self.filtered.len().saturating_sub(1));
    }

    /// Handle a key; needs the item slice to keep the ranking current.
    /// Enter returns `Submitted` with [`ComboboxState::highlighted`] set and
    /// copies the chosen item into the input.
    pub fn handle_key(&mut self, key: KeyEvent, items: &[&str]) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Down if self.open => {
                if self.highlight + 1 < self.filtered.len() {
                    self.highlight += 1;
                }
                Outcome::Consumed
            }
            KeyCode::Up if self.open => {
                self.highlight = self.highlight.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.open = true;
                self.filter(items);
                Outcome::Consumed
            }
            KeyCode::Enter if self.open => {
                if let Some(i) = self.highlighted() {
                    self.input.set_value(items[i]);
                    self.open = false;
                    return Outcome::Submitted;
                }
                self.open = false;
                Outcome::Consumed
            }
            KeyCode::Esc if self.open => {
                self.open = false;
                Outcome::Consumed
            }
            _ => {
                let out = self.input.handle_key(key);
                if out == Outcome::Changed {
                    self.open = true;
                    self.filter(items);
                    self.highlight = 0;
                }
                out
            }
        }
    }

    /// Paste entry point.
    pub fn insert_str(&mut self, s: &str, items: &[&str]) {
        self.input.insert_str(s);
        self.open = true;
        self.filter(items);
    }
}

/// The combobox view: an [`Input`] with a ranked dropdown that overdraws
/// below when open (render after the content beneath).
#[derive(Clone, Debug)]
pub struct Combobox<'a> {
    items: &'a [&'a str],
    placeholder: &'a str,
    focused: bool,
    disabled: bool,
    max_popup: u16,
    theme: Option<&'a Theme>,
}

impl<'a> Combobox<'a> {
    pub fn new(items: &'a [&'a str]) -> Combobox<'a> {
        Combobox {
            items,
            placeholder: "Search…",
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

impl<'a> StatefulWidget for Combobox<'a> {
    type State = ComboboxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ComboboxState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let field = Rect::new(area.x, area.y, area.width, 1);
        Input::new()
            .placeholder(self.placeholder)
            .focused(self.focused)
            .disabled(self.disabled)
            .theme(t)
            .render(field, buf, &mut state.input);

        if !state.open || self.disabled {
            return;
        }
        let rows = (state.filtered.len() as u16).min(self.max_popup);
        if rows == 0 {
            return;
        }
        let popup =
            Rect::new(area.x, area.y + 1, area.width, rows + 2).intersection(buf.area);
        if popup.height < 3 {
            return;
        }
        Clear.render(popup, buf);
        let block = Block::bordered()
            .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
            .style(Style::new().bg(t.bg[4]));
        let inner = block.inner(popup);
        block.render(popup, buf);
        state.view_h = inner.height as usize;
        if state.highlight < state.offset {
            state.offset = state.highlight;
        } else if state.highlight >= state.offset + state.view_h {
            state.offset = state.highlight + 1 - state.view_h;
        }
        for vis in 0..state.view_h {
            let fi = state.offset + vis;
            let Some(&item_idx) = state.filtered.get(fi) else { break };
            let y = inner.y + vis as u16;
            let is_cursor = fi == state.highlight;
            let mut style = Style::new().fg(t.fg[1]).bg(t.bg[4]);
            if is_cursor {
                style = Style::new().fg(t.fg[0]).bg(t.bg[3]).add_modifier(Modifier::BOLD);
                buf.set_style(Rect::new(inner.x, y, inner.width, 1), style);
            }
            buf.set_string(
                inner.x + 1,
                y,
                text::truncate(self.items[item_idx], inner.width.saturating_sub(2) as usize),
                style,
            );
        }
    }
}
