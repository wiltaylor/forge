use crate::event::{is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;
use std::collections::BTreeSet;

/// Scrollable single/multi-select list state. `highlight` is the keyboard
/// cursor; `selected` holds committed choices (at most one unless `multi`).
#[derive(Clone, Debug, Default)]
pub struct ListBoxState {
    pub highlight: usize,
    multi: bool,
    selected: BTreeSet<usize>,
    offset: usize,
    len: usize,
    view_h: usize,
}

impl ListBoxState {
    pub fn new() -> ListBoxState {
        ListBoxState::default()
    }

    pub fn multi() -> ListBoxState {
        ListBoxState { multi: true, ..Default::default() }
    }

    pub fn is_multi(&self) -> bool {
        self.multi
    }

    pub fn selected(&self) -> &BTreeSet<usize> {
        &self.selected
    }

    pub fn selected_one(&self) -> Option<usize> {
        self.selected.iter().next().copied()
    }

    pub fn is_selected(&self, i: usize) -> bool {
        self.selected.contains(&i)
    }

    pub fn select_only(&mut self, i: usize) {
        self.selected.clear();
        self.selected.insert(i);
    }

    pub fn toggle(&mut self, i: usize) {
        if !self.selected.remove(&i) {
            self.selected.insert(i);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    fn move_to(&mut self, target: usize) -> Outcome {
        let clamped = target.min(self.len.saturating_sub(1));
        if clamped != self.highlight {
            self.highlight = clamped;
        }
        Outcome::Consumed
    }

    /// ↑/↓/Home/End/PgUp/PgDn move the cursor; Space selects (or toggles in
    /// multi mode); Enter commits the highlighted item and submits.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) || self.len == 0 {
            return Outcome::Ignored;
        }
        let page = self.view_h.max(1);
        match key.code {
            KeyCode::Up => self.move_to(self.highlight.saturating_sub(1)),
            KeyCode::Down => self.move_to(self.highlight + 1),
            KeyCode::Home => self.move_to(0),
            KeyCode::End => self.move_to(usize::MAX),
            KeyCode::PageUp => self.move_to(self.highlight.saturating_sub(page)),
            KeyCode::PageDown => self.move_to(self.highlight.saturating_add(page)),
            KeyCode::Char(' ') => {
                if self.multi {
                    self.toggle(self.highlight);
                } else {
                    self.select_only(self.highlight);
                }
                Outcome::Changed
            }
            KeyCode::Enter => {
                if self.multi {
                    self.toggle(self.highlight);
                } else {
                    self.select_only(self.highlight);
                }
                Outcome::Submitted
            }
            _ => Outcome::Ignored,
        }
    }

    /// Move the cursor to the next item starting with `c` (type-ahead).
    pub fn jump_to(&mut self, items: &[&str], c: char) -> Outcome {
        let lc = c.to_ascii_lowercase();
        let n = items.len();
        for step in 1..=n {
            let i = (self.highlight + step) % n;
            if items[i].to_lowercase().starts_with(lc) {
                self.highlight = i;
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }
}

/// The list view. Highlight row on a raised surface, selections in accent;
/// a scrollbar appears when the list overflows.
#[derive(Clone, Debug)]
pub struct ListBox<'a> {
    items: &'a [&'a str],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> ListBox<'a> {
    pub fn new(items: &'a [&'a str]) -> ListBox<'a> {
        ListBox { items, focused: false, theme: None }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for ListBox<'a> {
    type State = ListBoxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListBoxState) {
        state.len = self.items.len();
        state.view_h = area.height as usize;
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        state.highlight = state.highlight.min(state.len.saturating_sub(1));
        // Keep the cursor in view.
        if state.highlight < state.offset {
            state.offset = state.highlight;
        } else if state.highlight >= state.offset + state.view_h {
            state.offset = state.highlight + 1 - state.view_h;
        }
        let overflow = state.len > state.view_h;
        let text_w = area.width.saturating_sub(if overflow { 1 } else { 0 });
        for (vis, i) in (state.offset..state.len.min(state.offset + state.view_h)).enumerate() {
            let y = area.y + vis as u16;
            let is_cursor = i == state.highlight;
            let is_selected = state.is_selected(i);
            let mut style = Style::new().fg(if is_selected { t.accent.fg } else { t.fg[1] });
            if is_cursor {
                style = style.bg(t.bg[3]).fg(if is_selected { t.accent.fg } else { t.fg[0] });
                if self.focused {
                    style = style.add_modifier(Modifier::BOLD);
                }
                buf.set_style(Rect::new(area.x, y, text_w, 1), style);
            }
            let mark = if state.multi {
                if is_selected { "✓ " } else { "  " }
            } else if is_selected {
                "● "
            } else {
                "  "
            };
            buf.set_string(area.x, y, mark, style);
            buf.set_string(
                area.x + 2,
                y,
                text::truncate(self.items[i], text_w.saturating_sub(2) as usize),
                style,
            );
        }
        if overflow {
            let x = area.x + area.width - 1;
            let track_h = area.height as usize;
            let thumb_h = (track_h * track_h / state.len).max(1);
            let thumb_top = state.offset * track_h / state.len;
            for dy in 0..area.height {
                let in_thumb = (dy as usize) >= thumb_top && (dy as usize) < thumb_top + thumb_h;
                let (ch, color) = if in_thumb {
                    ("█", t.border.strong)
                } else {
                    ("│", t.border.subtle)
                };
                buf.set_string(x, area.y + dy, ch, Style::new().fg(color));
            }
        }
    }
}
