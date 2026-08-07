//! `combobox` — ratatui.
//!
//! Pages: `reference/controls/combobox.md` and `reference/impl/ratatui/combobox.md`.
//!
//! A single-select field over a list too long to scan, where the user narrows by typing.
//! It composes `input` rather than reimplementing text editing.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Clear, StatefulWidget, Widget};

use crate::input::{Input, InputState};
use crate::outcome::Outcome;
use crate::theme::Theme;

/// The search glyph, the chevrons and the selection marker.
///
/// In a terminal a glyph *is* the icon — `anti-patterns.md` inverts "Unicode as icon"
/// here, and an icon font is what would be wrong.
const GLYPH_SEARCH: char = '/';
const GLYPH_CLOSED: char = '▾';
const GLYPH_OPEN: char = '▴';
const GLYPH_SELECTED: char = '>';
const GLYPH_MORE_ABOVE: char = '▴';
const GLYPH_MORE_BELOW: char = '▾';
const ELLIPSIS: char = '…';

/// One option.
///
/// `label` is what the user reads and what the filter matches. `value` is what the caller
/// stores. `hint` is an optional right-aligned word — a disabled option must not be
/// signalled by colour alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboBoxItem {
    pub value: String,
    pub label: String,
    pub disabled: bool,
    pub hint: Option<String>,
}

impl ComboBoxItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            hint: None,
        }
    }

    /// A disabled option cannot be committed. The popup stays open when one is tried.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// A short right-aligned word, so a disabled option reads as disabled without colour.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// The retained state. The app owns it; the widget is rebuilt each frame.
///
/// The four pieces of the control page map on to this platform as:
///
/// | Control page | Here |
/// |---|---|
/// | `open`   | `open` |
/// | `query`  | `input` plus `query_set` — unset and empty are different states |
/// | `active` | `highlight`, `None` for the page's `-1` |
/// | `value`  | `selected`, an index the caller reads and stores |
///
/// `view_h`, `offset` and `list_area` are private and exist only because a terminal
/// supplies no scrolling and no hit-testing.
#[derive(Debug, Clone, Default)]
pub struct ComboBoxState {
    /// The composed text field.
    pub input: InputState,
    /// The popup is showing.
    pub open: bool,
    /// `query` unset means "the field shows the selected label"; set means the user typed,
    /// and an empty query then shows every option. Collapsing the two blanks the field
    /// whenever the popup opens.
    query_set: bool,
    /// Which option the keyboard is on, as a position in `filtered`. `None` is "-1".
    highlight: Option<usize>,
    /// The committed selection, as an index into the caller's item slice.
    selected: Option<usize>,
    /// Item indices that survive the filter.
    filtered: Vec<usize>,
    /// The scroll window, recomputed at draw time from the area given.
    view_h: usize,
    offset: usize,
    /// Stored at draw time so the next mouse event can be tested against it.
    list_area: Rect,
}

impl ComboBoxState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The committed selection, as an index into the item slice. `None` when nothing is
    /// selected.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Commit a selection without a keystroke, and put its label in the field.
    pub fn select(&mut self, items: &[ComboBoxItem], index: Option<usize>) {
        self.selected = index.filter(|i| *i < items.len());
        self.reset_query(items);
    }

    /// How many options survive the filter. Zero is the empty state.
    pub fn matches(&self) -> usize {
        self.filtered.len()
    }

    /// Recompute the filter. Call it when the item slice changes.
    ///
    /// Case-insensitive substring on the option's **label**, never its value, and never a
    /// ranking. The ratatui page asks for a fuzzy ranking and marks that a Contract defect
    /// on itself; the Contract wins.
    pub fn refilter(&mut self, items: &[ComboBoxItem]) {
        self.filtered = if self.query_set {
            let needle = self.input.value().to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.label.to_lowercase().contains(&needle))
                .map(|(i, _)| i)
                .collect()
        } else {
            (0..items.len()).collect()
        };
        if let Some(h) = self.highlight {
            self.highlight = if self.filtered.is_empty() {
                None
            } else {
                Some(h.min(self.filtered.len() - 1))
            };
        }
    }

    /// Focus arrived: open the popup and select the existing text, so typing replaces it.
    pub fn focus(&mut self, items: &[ComboBoxItem]) {
        self.open = true;
        self.input.select_all();
        self.query_set = false;
        self.refilter(items);
        self.highlight_selected_or_first();
    }

    /// Focus left. Dismissing by any other route behaves as Escape.
    pub fn blur(&mut self, items: &[ComboBoxItem]) {
        self.dismiss(items);
    }

    /// Route one key. The caller decides this control has focus.
    ///
    /// Takes the item slice because the state does not own the items and the filter has to
    /// stay current. Reacts to key **presses** only.
    pub fn handle_key(&mut self, key: KeyEvent, items: &[ComboBoxItem]) -> Outcome {
        if key.kind != KeyEventKind::Press {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Down => {
                if !self.open {
                    self.open = true;
                    self.refilter(items);
                    self.highlight_selected_or_first();
                    return Outcome::Changed;
                }
                self.move_highlight(1)
            }
            KeyCode::Up => {
                // Closed, Up is not ours: the parent keeps routing, so a form can move
                // focus to the field above.
                if !self.open {
                    return Outcome::Ignored;
                }
                self.move_highlight(-1)
            }
            KeyCode::Enter => {
                if !self.open {
                    return Outcome::Ignored;
                }
                self.commit(items)
            }
            KeyCode::Esc => {
                // Closed, Escape is not ours. Escape closes the innermost open thing, one
                // layer per press.
                if !self.open {
                    return Outcome::Ignored;
                }
                self.dismiss(items);
                Outcome::Cancelled
            }
            _ => match self.input.handle_key(key) {
                Outcome::Changed => {
                    // Typing sets query, opens the popup, and puts active on 0.
                    self.query_set = true;
                    self.open = true;
                    self.refilter(items);
                    self.highlight = if self.filtered.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                    Outcome::Changed
                }
                other => other,
            },
        }
    }

    /// Route one mouse event. Hit-testing is manual — there is no DOM to ask.
    pub fn handle_mouse(&mut self, event: MouseEvent, items: &[ComboBoxItem]) -> Outcome {
        if !self.open || !contains(self.list_area, event.column, event.row) {
            return Outcome::Ignored;
        }
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let row = (event.row - self.list_area.y) as usize;
                let position = self.offset + row;
                if position >= self.filtered.len() {
                    return Outcome::Consumed;
                }
                self.highlight = Some(position);
                self.commit(items)
            }
            // Wheel moves the cursor, not an independent scroll offset — one cursor,
            // always visible.
            MouseEventKind::ScrollDown => self.move_highlight(1),
            MouseEventKind::ScrollUp => self.move_highlight(-1),
            _ => Outcome::Ignored,
        }
    }

    fn move_highlight(&mut self, delta: isize) -> Outcome {
        if self.filtered.is_empty() {
            return Outcome::Consumed;
        }
        let last = self.filtered.len() - 1;
        let next = match self.highlight {
            // Down stops at the last option and does not wrap. Up stops at the first.
            None => {
                if delta > 0 {
                    0
                } else {
                    last
                }
            }
            Some(current) => {
                if delta > 0 {
                    (current + 1).min(last)
                } else {
                    current.saturating_sub(1)
                }
            }
        };
        if self.highlight == Some(next) {
            return Outcome::Consumed;
        }
        self.highlight = Some(next);
        Outcome::Changed
    }

    fn commit(&mut self, items: &[ComboBoxItem]) -> Outcome {
        let Some(index) = self.highlight.and_then(|h| self.filtered.get(h).copied()) else {
            return Outcome::Consumed;
        };
        // Committing a disabled option is a no-op, and the popup stays open.
        if items[index].disabled {
            return Outcome::Consumed;
        }
        self.selected = Some(index);
        self.open = false;
        self.reset_query(items);
        Outcome::Submitted
    }

    fn dismiss(&mut self, items: &[ComboBoxItem]) {
        self.open = false;
        self.reset_query(items);
    }

    /// Clear `query` and put the selected label back in the field.
    ///
    /// A terminal field has no separate label slot, so Enter and Escape reach the
    /// Contract's visible result by writing the label into the composed input.
    fn reset_query(&mut self, items: &[ComboBoxItem]) {
        self.query_set = false;
        match self.selected.and_then(|i| items.get(i)) {
            Some(item) => self.input.set_value(&item.label),
            None => self.input.clear(),
        }
        self.refilter(items);
        self.highlight_selected_or_first();
    }

    fn highlight_selected_or_first(&mut self) {
        self.highlight = match self.selected {
            Some(selected) => self.filtered.iter().position(|i| *i == selected),
            None => None,
        };
    }

    /// Keep the highlighted row inside the scroll window.
    fn clamp_scroll(&mut self) {
        let len = self.filtered.len();
        if self.view_h == 0 || len == 0 {
            self.offset = 0;
            return;
        }
        let max_offset = len.saturating_sub(self.view_h);
        if let Some(h) = self.highlight {
            if h < self.offset {
                self.offset = h;
            } else if h >= self.offset + self.view_h {
                self.offset = h + 1 - self.view_h;
            }
        }
        self.offset = self.offset.min(max_offset);
    }
}

/// The `combobox` widget: a field, and a popup below it.
pub struct ComboBox<'a> {
    items: &'a [ComboBoxItem],
    theme: &'a Theme,
    focused: bool,
    /// Shown in the field while it is empty.
    placeholder: &'a str,
    /// The single line the popup holds when the filter matches nothing.
    empty_text: &'a str,
    /// The tallest the popup may be. It is also capped by the rows left below the field.
    max_rows: u16,
}

impl<'a> ComboBox<'a> {
    pub fn new(items: &'a [ComboBoxItem], theme: &'a Theme) -> Self {
        Self {
            items,
            theme,
            focused: false,
            placeholder: "",
            empty_text: "",
            max_rows: 8,
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

    /// Every visible string is a parameter, this one included.
    pub fn empty_text(mut self, empty_text: &'a str) -> Self {
        self.empty_text = empty_text;
        self
    }

    pub fn max_rows(mut self, max_rows: u16) -> Self {
        self.max_rows = max_rows.max(1);
        self
    }

    /// The narrowest useful width: the focus gutter, the field padding, the glyphs.
    pub fn width(&self) -> u16 {
        let longest = self
            .items
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(0) as u16;
        GUTTER + FIELD_CHROME + longest.max(self.placeholder.chars().count() as u16)
    }
}

/// The focus gutter, kept whatever the focus state, so the field never shifts sideways.
const GUTTER: u16 = 2;
/// Pad, search glyph, pad, … , chevron, pad.
const FIELD_CHROME: u16 = 5;
/// The popup's selection marker column.
const MARKER: u16 = 2;

impl StatefulWidget for ComboBox<'_> {
    type State = ComboBoxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ComboBoxState) {
        if area.height == 0 || area.width <= GUTTER + FIELD_CHROME {
            return;
        }
        let theme = self.theme;
        let field_row = Rect {
            height: 1,
            ..area
        };

        // Focus is drawn, not owned: a `>` in the gutter, as laws.md requires.
        buf.set_string(
            field_row.x,
            field_row.y,
            if self.focused {
                format!("{GLYPH_SELECTED} ")
            } else {
                "  ".to_string()
            },
            Style::default().fg(theme.accent.base).bg(theme.bg[0]),
        );

        let field = Rect {
            x: field_row.x + GUTTER,
            y: field_row.y,
            width: field_row.width - GUTTER,
            height: 1,
        };
        buf.set_style(field, Style::default().bg(theme.bg[1]).fg(theme.fg[0]));

        let glyph_style = Style::default().bg(theme.bg[1]).fg(if self.focused {
            theme.accent.base
        } else {
            theme.fg[2]
        });
        buf.set_string(field.x + 1, field.y, GLYPH_SEARCH.to_string(), glyph_style);
        buf.set_string(
            field.right() - 2,
            field.y,
            if state.open {
                GLYPH_OPEN.to_string()
            } else {
                GLYPH_CLOSED.to_string()
            },
            glyph_style,
        );

        let text_area = Rect {
            x: field.x + 3,
            y: field.y,
            width: field.width - FIELD_CHROME,
            height: 1,
        };
        Input::new(theme)
            .focused(self.focused)
            .placeholder(self.placeholder)
            .render(text_area, buf, &mut state.input);

        if !state.open {
            state.list_area = Rect::ZERO;
            state.view_h = 0;
            return;
        }

        // The popup never flips above the field. In a terminal the layout picks the field
        // position, and flipping makes the control jump between frames.
        let below = buf.area().bottom().saturating_sub(field_row.y + 1);
        if below == 0 || field.width < MARKER + 2 {
            state.list_area = Rect::ZERO;
            state.view_h = 0;
            return;
        }
        let wanted = state.filtered.len().max(1) as u16;
        let height = wanted.min(self.max_rows).min(below);
        let popup = Rect {
            x: field.x,
            y: field_row.y + 1,
            width: field.width,
            height,
        };

        // A terminal cell has no z-order, so the content underneath shows through without
        // this.
        Clear.render(popup, buf);
        buf.set_style(popup, Style::default().bg(theme.bg[4]).fg(theme.fg[0]));

        state.list_area = popup;
        state.view_h = height as usize;
        state.clamp_scroll();

        if state.filtered.is_empty() {
            // The empty state says what to do next, and it is the caller's string.
            let text = truncate(self.empty_text, (popup.width - MARKER) as usize);
            buf.set_string(
                popup.x + MARKER,
                popup.y,
                text,
                Style::default().bg(theme.bg[4]).fg(theme.fg[2]),
            );
            return;
        }

        let label_width = popup.width.saturating_sub(MARKER + 1) as usize;
        for row in 0..height as usize {
            let position = state.offset + row;
            let Some(&index) = state.filtered.get(position) else {
                break;
            };
            let item = &self.items[index];
            let y = popup.y + row as u16;
            let active = state.highlight == Some(position);

            let fg = if item.disabled {
                theme.fg[3]
            } else {
                theme.fg[0]
            };
            let mut row_style = Style::default().bg(theme.bg[4]).fg(fg);
            if active {
                // A reversed style, never a background colour alone: 256-colour terminals
                // collapse near colours and the row disappears.
                row_style = row_style.reversed();
            }
            buf.set_style(
                Rect {
                    x: popup.x,
                    y,
                    width: popup.width,
                    height: 1,
                },
                row_style,
            );

            // Selection is a `>` in the left gutter, not a check glyph.
            if state.selected == Some(index) {
                buf.set_string(popup.x, y, GLYPH_SELECTED.to_string(), row_style);
            }

            // A hint is a word, so "unavailable" never rides on colour alone.
            let hint = item.hint.as_deref().unwrap_or("");
            let hint_width = hint.chars().count().min(label_width);
            let room = label_width - hint_width.min(label_width);
            buf.set_string(
                popup.x + MARKER,
                y,
                truncate(&item.label, room.saturating_sub(1)),
                row_style,
            );
            if hint_width > 0 {
                buf.set_string(
                    popup.x + MARKER + room as u16,
                    y,
                    truncate(hint, hint_width),
                    if active {
                        row_style
                    } else {
                        row_style.fg(theme.fg[2])
                    },
                );
            }
        }

        // No scrollbar exists in a terminal; these two glyphs are the whole of it.
        let more = Style::default().bg(theme.bg[4]).fg(theme.fg[2]);
        if state.offset > 0 {
            buf.set_string(popup.right() - 1, popup.y, GLYPH_MORE_ABOVE.to_string(), more);
        }
        if state.offset + state.view_h < state.filtered.len() {
            buf.set_string(
                popup.right() - 1,
                popup.bottom() - 1,
                GLYPH_MORE_BELOW.to_string(),
                more,
            );
        }
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    area.width > 0
        && area.height > 0
        && column >= area.x
        && column < area.right()
        && row >= area.y
        && row < area.bottom()
}

/// Truncate with an ellipsis at the end. ratatui wraps instead, and a wrapped row breaks
/// the fixed row height.
fn truncate(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
    out.push(ELLIPSIS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn items() -> Vec<ComboBoxItem> {
        vec![
            ComboBoxItem::new("us-east-1", "us-east-1 · N. Virginia"),
            ComboBoxItem::new("eu-west-2", "eu-west-2 · London"),
            ComboBoxItem::new("ap-east-1", "ap-east-1 · Hong Kong")
                .disabled(true)
                .hint("unavailable"),
        ]
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn nothing_is_selected_at_the_start() {
        let state = ComboBoxState::new();
        assert_eq!(state.selected(), None);
        assert!(!state.open);
    }

    #[test]
    fn typing_opens_filters_and_puts_active_on_zero() {
        let items = items();
        let mut state = ComboBoxState::new();
        state.refilter(&items);
        assert_eq!(state.handle_key(press(KeyCode::Char('l')), &items), Outcome::Changed);
        assert!(state.open);
        assert_eq!(state.matches(), 1);
        assert_eq!(state.highlight, Some(0));
    }

    #[test]
    fn the_filter_is_a_case_insensitive_substring_and_not_a_ranking() {
        let items = items();
        let mut state = ComboBoxState::new();
        state.handle_key(press(KeyCode::Char('E')), &items);
        // Substring, in item order: every label contains an "e".
        assert_eq!(state.filtered, vec![0, 1, 2]);
    }

    #[test]
    fn no_match_leaves_an_empty_list_and_no_active_row() {
        let items = items();
        let mut state = ComboBoxState::new();
        for c in "zzz".chars() {
            state.handle_key(press(KeyCode::Char(c)), &items);
        }
        assert_eq!(state.matches(), 0);
        assert_eq!(state.highlight, None);
    }

    #[test]
    fn enter_commits_closes_and_clears_the_query() {
        let items = items();
        let mut state = ComboBoxState::new();
        state.handle_key(press(KeyCode::Char('l')), &items);
        assert_eq!(state.handle_key(press(KeyCode::Enter), &items), Outcome::Submitted);
        assert_eq!(state.selected(), Some(1));
        assert!(!state.open);
        // query cleared: the field shows the selected label, not the search string.
        assert_eq!(state.input.value(), "eu-west-2 · London");
        // query unset, not empty: every option is listed again.
        assert_eq!(state.matches(), 3);
    }

    #[test]
    fn escape_closes_clears_the_query_and_keeps_the_value() {
        let items = items();
        let mut state = ComboBoxState::new();
        state.handle_key(press(KeyCode::Char('l')), &items);
        state.handle_key(press(KeyCode::Enter), &items);
        state.handle_key(press(KeyCode::Char('z')), &items);
        assert_eq!(state.handle_key(press(KeyCode::Esc), &items), Outcome::Cancelled);
        assert!(!state.open);
        assert_eq!(state.selected(), Some(1));
        assert_eq!(state.input.value(), "eu-west-2 · London");
    }

    #[test]
    fn a_closed_combobox_ignores_escape_so_it_never_closes_two_layers() {
        let items = items();
        let mut state = ComboBoxState::new();
        assert_eq!(state.handle_key(press(KeyCode::Esc), &items), Outcome::Ignored);
    }

    #[test]
    fn an_empty_query_is_not_an_unset_query() {
        let items = items();
        let mut state = ComboBoxState::new();
        state.handle_key(press(KeyCode::Char('l')), &items);
        state.handle_key(press(KeyCode::Enter), &items);
        assert_eq!(state.input.value(), "eu-west-2 · London");
        // Clearing by hand is the empty query: field blank, every option listed.
        state.handle_key(press(KeyCode::Char('x')), &items);
        for _ in 0..40 {
            state.handle_key(press(KeyCode::Backspace), &items);
        }
        assert!(state.input.is_empty());
        assert_eq!(state.matches(), 3);
    }

    #[test]
    fn down_stops_at_the_last_option_and_up_at_the_first() {
        let items = items();
        let mut state = ComboBoxState::new();
        state.handle_key(press(KeyCode::Down), &items);
        assert!(state.open);
        for _ in 0..10 {
            state.handle_key(press(KeyCode::Down), &items);
        }
        assert_eq!(state.highlight, Some(2));
        for _ in 0..10 {
            state.handle_key(press(KeyCode::Up), &items);
        }
        assert_eq!(state.highlight, Some(0));
    }

    #[test]
    fn committing_a_disabled_option_is_a_no_op_and_the_popup_stays_open() {
        let items = items();
        let mut state = ComboBoxState::new();
        // The first Down opens; the rest move, and stop at the last option.
        for _ in 0..6 {
            state.handle_key(press(KeyCode::Down), &items);
        }
        assert_eq!(state.highlight, Some(2));
        assert_eq!(state.handle_key(press(KeyCode::Enter), &items), Outcome::Consumed);
        assert_eq!(state.selected(), None);
        assert!(state.open);
    }

    #[test]
    fn key_release_is_ignored_so_nothing_fires_twice() {
        let items = items();
        let mut state = ComboBoxState::new();
        let mut key = press(KeyCode::Down);
        key.kind = KeyEventKind::Release;
        assert_eq!(state.handle_key(key, &items), Outcome::Ignored);
        assert!(!state.open);
    }

    #[test]
    fn the_empty_line_is_drawn_when_the_filter_matches_nothing() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let items = items();
        let theme = Theme::dark();
        let mut state = ComboBoxState::new();
        for c in "zzz".chars() {
            state.handle_key(press(KeyCode::Char(c)), &items);
        }
        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        terminal
            .draw(|frame| {
                frame.render_stateful_widget(
                    ComboBox::new(&items, &theme)
                        .focused(true)
                        .empty_text("No region matches"),
                    Rect::new(0, 0, 40, 1),
                    &mut state,
                );
            })
            .unwrap();
        let rendered = format!("{:?}", terminal.backend().buffer());
        assert!(rendered.contains("No region matches"), "{rendered}");
    }
}
