use crate::event::{in_area, is_press, left_down, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Right,
}

/// Column definition: title, fixed width (`0` = share the remaining space),
/// alignment, and whether `s` may sort on it.
#[derive(Clone, Copy, Debug)]
pub struct Column<'a> {
    pub title: &'a str,
    pub width: u16,
    pub align: Align,
    pub sortable: bool,
}

impl<'a> Column<'a> {
    pub fn new(title: &'a str) -> Column<'a> {
        Column {
            title,
            width: 0,
            align: Align::Left,
            sortable: true,
        }
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn right(mut self) -> Self {
        self.align = Align::Right;
        self
    }

    pub fn fixed(mut self) -> Self {
        self.sortable = false;
        self
    }
}

/// Cursor, row selection, and the sort request. Sorting the actual rows is
/// the app's job — the state carries `(column, ascending)` and the header
/// shows the arrow; re-order your row slice when `handle_key` returns
/// `Changed` with a new `sort`.
#[derive(Clone, Debug, Default)]
pub struct TableState {
    pub cursor: usize,
    pub sort: Option<(usize, bool)>,
    selected: BTreeSet<usize>,
    offset: usize,
    len: usize,
    cols: usize,
    view_h: usize,
    area: Rect,
    col_spans: Vec<(u16, u16)>,
}

impl TableState {
    pub fn new() -> TableState {
        TableState::default()
    }

    pub fn selected(&self) -> &BTreeSet<usize> {
        &self.selected
    }

    pub fn is_selected(&self, row: usize) -> bool {
        self.selected.contains(&row)
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    fn move_to(&mut self, target: usize) -> Outcome {
        self.cursor = target.min(self.len.saturating_sub(1));
        Outcome::Consumed
    }

    /// Click a header cell to sort by that column (toggling direction),
    /// click a row to move the cursor, wheel to scroll.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        let delta = scroll_delta(ev);
        if delta != 0 {
            return self.move_to(if delta < 0 {
                self.cursor.saturating_sub(1)
            } else {
                self.cursor + 1
            });
        }
        if !left_down(ev) {
            return Outcome::Ignored;
        }
        if ev.row == self.area.y {
            // Header: sort by the clicked column.
            for (ci, (x0, x1)) in self.col_spans.iter().enumerate() {
                if (*x0..*x1).contains(&ev.column) {
                    self.sort = match self.sort {
                        Some((c, asc)) if c == ci => Some((ci, !asc)),
                        _ => Some((ci, true)),
                    };
                    return Outcome::Changed;
                }
            }
            return Outcome::Consumed;
        }
        let row = self.offset + (ev.row - self.area.y - 1) as usize;
        if row < self.len {
            self.cursor = row;
            return Outcome::Consumed;
        }
        Outcome::Ignored
    }

    /// ↑/↓/PgUp/PgDn/Home/End move; Space toggles row selection; `s` cycles
    /// the sort (col0 ↑ → col0 ↓ → col1 ↑ → … → off); Enter submits the
    /// cursor row.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) || self.len == 0 {
            return Outcome::Ignored;
        }
        let page = self.view_h.max(1);
        match key.code {
            KeyCode::Up => self.move_to(self.cursor.saturating_sub(1)),
            KeyCode::Down => self.move_to(self.cursor + 1),
            KeyCode::Home => self.move_to(0),
            KeyCode::End => self.move_to(usize::MAX),
            KeyCode::PageUp => self.move_to(self.cursor.saturating_sub(page)),
            KeyCode::PageDown => self.move_to(self.cursor.saturating_add(page)),
            KeyCode::Char(' ') => {
                if !self.selected.remove(&self.cursor) {
                    self.selected.insert(self.cursor);
                }
                Outcome::Changed
            }
            KeyCode::Char('s') => {
                self.sort = match self.sort {
                    None => Some((0, true)),
                    Some((c, true)) => Some((c, false)),
                    Some((c, false)) if c + 1 < self.cols => Some((c + 1, true)),
                    Some(_) => None,
                };
                Outcome::Changed
            }
            KeyCode::Enter => Outcome::Submitted,
            _ => Outcome::Ignored,
        }
    }
}

/// Flat, border-free data table: raised sticky header with sort arrows,
/// cursor row on a raised surface, Space-selected rows tinted accent.
#[derive(Clone, Debug)]
pub struct Table<'a> {
    columns: &'a [Column<'a>],
    rows: &'a [Vec<&'a str>],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Table<'a> {
    pub fn new(columns: &'a [Column<'a>], rows: &'a [Vec<&'a str>]) -> Table<'a> {
        Table {
            columns,
            rows,
            focused: false,
            theme: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn widths(&self, total: u16) -> Vec<u16> {
        let gap = 2u16;
        let fixed: u16 = self.columns.iter().map(|c| c.width).sum();
        let gaps = gap * self.columns.len().saturating_sub(1) as u16;
        let fills = self.columns.iter().filter(|c| c.width == 0).count() as u16;
        let spare = total.saturating_sub(fixed + gaps);
        let fill_w = if fills > 0 { spare / fills } else { 0 };
        self.columns
            .iter()
            .map(|c| if c.width == 0 { fill_w } else { c.width })
            .collect()
    }

    fn put_cell(buf: &mut Buffer, x: u16, y: u16, w: u16, value: &str, align: Align, style: Style) {
        let value = text::truncate(value, w as usize);
        let vw = text::width(&value) as u16;
        let x = match align {
            Align::Left => x,
            Align::Right => x + w.saturating_sub(vw),
        };
        buf.set_string(x, y, value, style);
    }
}

impl<'a> StatefulWidget for Table<'a> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        state.len = self.rows.len();
        state.cols = self.columns.len();
        state.view_h = area.height.saturating_sub(1) as usize;
        state.area = area;
        if area.is_empty() || self.columns.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let widths = self.widths(area.width);
        state.col_spans = {
            let mut spans = Vec::with_capacity(widths.len());
            let mut cx = area.x;
            for w in &widths {
                spans.push((cx, cx + w + 2));
                cx += w + 2;
            }
            spans
        };

        // Header (sticky by construction).
        let header_style = Style::new()
            .fg(t.fg[1])
            .bg(t.bg[2])
            .add_modifier(Modifier::BOLD);
        buf.set_style(Rect::new(area.x, area.y, area.width, 1), header_style);
        let mut x = area.x;
        for (i, (col, w)) in self.columns.iter().zip(&widths).enumerate() {
            let arrow = match state.sort {
                Some((c, asc)) if c == i => {
                    if asc {
                        " ▲"
                    } else {
                        " ▼"
                    }
                }
                _ => "",
            };
            let title = format!("{}{}", col.title, arrow);
            let style = if arrow.is_empty() {
                header_style
            } else {
                header_style.fg(t.accent.fg)
            };
            Table::put_cell(buf, x, area.y, *w, &title, col.align, style);
            x += w + 2;
        }

        if state.view_h == 0 {
            return;
        }
        state.cursor = state.cursor.min(state.len.saturating_sub(1));
        if state.cursor < state.offset {
            state.offset = state.cursor;
        } else if state.cursor >= state.offset + state.view_h {
            state.offset = state.cursor + 1 - state.view_h;
        }

        for (vis, ri) in (state.offset..state.len.min(state.offset + state.view_h)).enumerate() {
            let y = area.y + 1 + vis as u16;
            let is_cursor = ri == state.cursor;
            let is_selected = state.is_selected(ri);
            let mut row_style = Style::new().fg(t.fg[1]);
            if is_selected {
                row_style = row_style.fg(t.accent.fg).bg(t.accent.bg);
            }
            if is_cursor {
                row_style = row_style.fg(t.fg[0]).bg(t.bg[3]);
                if self.focused {
                    row_style = row_style.add_modifier(Modifier::BOLD);
                }
            }
            if is_cursor || is_selected {
                buf.set_style(Rect::new(area.x, y, area.width, 1), row_style);
            }
            let mut x = area.x;
            for (ci, (col, w)) in self.columns.iter().zip(&widths).enumerate() {
                let value = self.rows[ri].get(ci).copied().unwrap_or("");
                Table::put_cell(buf, x, y, *w, value, col.align, row_style);
                x += w + 2;
            }
        }
    }
}
