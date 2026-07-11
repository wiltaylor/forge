//! Soft-wrapping plain-text editor over one markdown source string — the
//! focused-block editing engine of the block editor. Wrapping is greedy word
//! wrap by display cells (the same rule as `forge_blocks::wrap_spans`), so
//! the raw source occupies exactly the rows the renderer draws. Cursor moves
//! are grapheme-aware; vertical moves preserve a desired visual column.

use crate::text;
use unicode_segmentation::UnicodeSegmentation;

/// Wrap `src` at `width` cells into byte ranges, one per visual row. Ranges
/// cover every byte except the `\n` separators; a row that ends at a soft
/// break shares its end offset with the next row's start. `width == 0`
/// disables soft wrap (one row per hard line).
fn wrap_ranges(src: &str, width: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut seg_start = 0usize;
    for seg in src.split('\n') {
        let seg_end = seg_start + seg.len();
        if width == 0 || text::width(seg) <= width {
            out.push((seg_start, seg_end));
        } else {
            // Tokens are words keeping their trailing space (wrap_spans rule:
            // trailing spaces may hang past the edge without forcing a break).
            let mut tokens: Vec<(usize, usize)> = Vec::new();
            let mut start = seg_start;
            for (i, ch) in seg.char_indices() {
                if ch == ' ' {
                    tokens.push((start, seg_start + i + 1));
                    start = seg_start + i + 1;
                }
            }
            if start < seg_end {
                tokens.push((start, seg_end));
            }
            let mut line_start = seg_start;
            let mut used = 0usize;
            for (ts, te) in tokens {
                let tok = &src[ts..te];
                let vis = text::width(tok.trim_end_matches(' '));
                if used > 0 && used + vis > width {
                    out.push((line_start, ts));
                    line_start = ts;
                    used = 0;
                }
                if vis > width {
                    // A single token wider than the row: split by grapheme.
                    for (gi, g) in tok.grapheme_indices(true) {
                        let gw = text::width(g);
                        if used > 0 && used + gw > width {
                            out.push((line_start, ts + gi));
                            line_start = ts + gi;
                            used = 0;
                        }
                        used += gw;
                    }
                } else {
                    used += text::width(tok);
                }
            }
            out.push((line_start, seg_end));
        }
        seg_start = seg_end + 1;
    }
    out
}

/// Single-string plain-text editor with soft wrap. The caller sets the
/// content width at layout time; all cursor math then agrees with what the
/// renderer draws (raw monospace source, no rich styling).
#[derive(Clone, Debug)]
pub(crate) struct WrapEdit {
    src: String,
    /// Cursor byte offset (always a char boundary).
    cursor: usize,
    /// Preferred display column (cells) for vertical moves.
    desired: usize,
    /// Content width in cells from the last layout; 0 = no soft wrap.
    width: usize,
}

impl WrapEdit {
    pub fn new(src: impl Into<String>, cursor: usize) -> WrapEdit {
        let src = src.into();
        let mut we = WrapEdit {
            src,
            cursor: 0,
            desired: 0,
            width: 0,
        };
        we.set_cursor(cursor);
        we
    }

    pub fn src(&self) -> &str {
        &self.src
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    pub fn set_cursor(&mut self, byte: usize) {
        let mut b = byte.min(self.src.len());
        while b > 0 && !self.src.is_char_boundary(b) {
            b -= 1;
        }
        self.cursor = b;
        self.sync_desired();
    }

    /// Visual rows as byte ranges at the current width.
    pub fn lines(&self) -> Vec<(usize, usize)> {
        wrap_ranges(&self.src, self.width)
    }

    pub fn rows(&self) -> usize {
        self.lines().len()
    }

    /// Cursor position as (visual row, display column in cells). A cursor
    /// sitting exactly on a soft-break boundary belongs to the next row
    /// (column 0), matching where the renderer draws it.
    pub fn pos(&self) -> (usize, usize) {
        let lines = self.lines();
        for (i, &(s, e)) in lines.iter().enumerate() {
            let soft = lines.get(i + 1).is_some_and(|&(ns, _)| ns == e);
            if self.cursor < e || (self.cursor == e && !soft) {
                return (i, text::width(&self.src[s..self.cursor.min(e)]));
            }
        }
        let &(s, e) = lines.last().expect("wrap_ranges yields at least one row");
        (lines.len() - 1, text::width(&self.src[s..e]))
    }

    /// Byte offset of the grapheme at (row, col cells), clamped to the row.
    pub fn byte_at(&self, row: usize, col: usize) -> usize {
        let lines = self.lines();
        let row = row.min(lines.len() - 1);
        let (s, e) = lines[row];
        let mut w = 0usize;
        for (i, g) in self.src[s..e].grapheme_indices(true) {
            let gw = text::width(g);
            if w + gw > col {
                return s + i;
            }
            w += gw;
        }
        e
    }

    fn prev_boundary(&self, byte: usize) -> usize {
        self.src[..byte]
            .grapheme_indices(true)
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_boundary(&self, byte: usize) -> usize {
        self.src[byte..]
            .graphemes(true)
            .next()
            .map(|g| byte + g.len())
            .unwrap_or(self.src.len())
    }

    fn sync_desired(&mut self) {
        self.desired = self.pos().1;
    }

    /// Place the cursor on `row` at the desired column. A landing spot that
    /// coincides with a soft-break boundary would belong to the next row, so
    /// it steps back one grapheme to stay on the target row.
    fn move_to_row(&mut self, row: usize) {
        self.cursor = self.byte_at(row, self.desired);
        if self.pos().0 != row {
            self.cursor = self.prev_boundary(self.cursor);
        }
    }

    /// Place the cursor at (row, col cells) and adopt `col` as the desired
    /// column (mouse placement).
    pub fn move_to(&mut self, row: usize, col: usize) {
        self.desired = col;
        self.move_to_row(row.min(self.rows().saturating_sub(1)));
    }

    pub fn insert(&mut self, s: &str) {
        self.src.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.sync_desired();
    }

    /// Replace `start..end` with `s`; the cursor lands after the insertion.
    pub fn replace_range(&mut self, start: usize, end: usize, s: &str) {
        self.src.replace_range(start..end, s);
        self.cursor = start + s.len();
        self.sync_desired();
    }

    /// Delete the grapheme before the cursor. False when already at byte 0
    /// (the caller merges with the previous block).
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let to = self.prev_boundary(self.cursor);
        self.src.replace_range(to..self.cursor, "");
        self.cursor = to;
        self.sync_desired();
        true
    }

    /// Delete the grapheme after the cursor. False at the end of the source.
    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.src.len() {
            return false;
        }
        let to = self.next_boundary(self.cursor);
        self.src.replace_range(self.cursor..to, "");
        true
    }

    pub fn left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.prev_boundary(self.cursor);
        }
        self.sync_desired();
    }

    pub fn right(&mut self) {
        if self.cursor < self.src.len() {
            self.cursor = self.next_boundary(self.cursor);
        }
        self.sync_desired();
    }

    pub fn home(&mut self) {
        let (row, _) = self.pos();
        self.cursor = self.lines()[row].0;
        self.sync_desired();
    }

    pub fn end(&mut self) {
        let (row, _) = self.pos();
        self.cursor = self.lines()[row].1;
        if self.pos().0 != row {
            self.cursor = self.prev_boundary(self.cursor);
        }
        self.sync_desired();
    }

    /// Move up one visual row. False on the first row (the caller moves
    /// focus to the previous block, carrying the desired column).
    pub fn up(&mut self) -> bool {
        let (row, _) = self.pos();
        if row == 0 {
            return false;
        }
        self.move_to_row(row - 1);
        true
    }

    /// Move down one visual row. False on the last row.
    pub fn down(&mut self) -> bool {
        let (row, _) = self.pos();
        if row + 1 >= self.rows() {
            return false;
        }
        self.move_to_row(row + 1);
        true
    }

    pub fn desired(&self) -> usize {
        self.desired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn we(src: &str, cursor: usize, width: usize) -> WrapEdit {
        let mut w = WrapEdit::new(src, cursor);
        w.set_width(width);
        w
    }

    #[test]
    fn wraps_ascii_at_width_8() {
        let w = we("alpha beta gamma", 0, 8);
        let lines = w.lines();
        assert_eq!(lines, vec![(0, 6), (6, 11), (11, 16)]);
    }

    #[test]
    fn width_40_keeps_one_row() {
        let w = we("alpha beta gamma", 16, 40);
        assert_eq!(w.rows(), 1);
        assert_eq!(w.pos(), (0, 16));
    }

    #[test]
    fn hard_newlines_force_rows() {
        let w = we("a\n\nb", 4, 40);
        assert_eq!(w.lines(), vec![(0, 1), (2, 2), (3, 4)]);
        assert_eq!(w.pos(), (2, 1));
    }

    #[test]
    fn pos_maps_multibyte_columns() {
        // "é" is 2 bytes / 1 cell; "你" is 3 bytes / 2 cells.
        let w = we("é你x", 0, 40);
        assert_eq!(we("é你x", 2, 40).pos(), (0, 1));
        assert_eq!(we("é你x", 5, 40).pos(), (0, 3));
        assert_eq!(w.byte_at(0, 1), 2);
        assert_eq!(w.byte_at(0, 2), 2); // mid-CJK clamps to its start
        assert_eq!(w.byte_at(0, 3), 5);
    }

    #[test]
    fn cjk_wraps_by_cells_not_bytes() {
        let w = we("你好世界", 0, 4);
        assert_eq!(w.lines(), vec![(0, 6), (6, 12)]);
        assert_eq!(we("你好世界", 6, 4).pos(), (1, 0));
    }

    #[test]
    fn emoji_counts_two_cells() {
        let mut w = we("a🚀b", 0, 8);
        assert_eq!(we("a🚀b", 5, 8).pos(), (0, 3));
        w.right();
        w.right();
        assert_eq!(w.cursor(), 5); // grapheme-wise past the emoji
    }

    #[test]
    fn soft_boundary_cursor_belongs_to_next_row() {
        // Rows: "alpha " (0..6), "beta" (6..10); byte 6 is row 1 col 0.
        let w = we("alpha beta", 6, 8);
        assert_eq!(w.pos(), (1, 0));
    }

    #[test]
    fn up_down_preserve_desired_column() {
        let mut w = we("alpha beta\nxy\nlonger line", 0, 40);
        w.set_cursor(8); // row 0, col 8
        assert!(w.down());
        assert_eq!(w.pos(), (1, 2)); // clamped to "xy" end
        assert!(w.down());
        assert_eq!(w.pos(), (2, 8)); // desired column restored
        assert!(w.up());
        assert!(w.up());
        assert_eq!(w.pos(), (0, 8));
        assert!(!w.up());
    }

    #[test]
    fn down_onto_soft_row_stays_on_row() {
        // width 4 rows of "aaaa bbbb": "aaaa " / "bbbb".
        let mut w = we("aaaa bbbb", 4, 4);
        assert_eq!(w.pos(), (0, 4));
        assert!(w.down());
        assert_eq!(w.pos().0, 1);
        assert!(!w.down());
    }

    #[test]
    fn insert_and_backspace_at_boundaries() {
        let mut w = we("", 0, 8);
        assert!(!w.backspace());
        w.insert("你");
        w.insert("a");
        assert_eq!(w.src(), "你a");
        assert_eq!(w.cursor(), 4);
        assert!(w.backspace());
        assert!(w.backspace());
        assert_eq!(w.src(), "");
        assert!(!w.delete());
        w.insert("🚀");
        w.set_cursor(0);
        assert!(w.delete());
        assert_eq!(w.src(), "");
    }

    #[test]
    fn replace_range_moves_cursor_after_insertion() {
        let mut w = we("go :ro now", 0, 40);
        w.replace_range(3, 6, ":rocket:");
        assert_eq!(w.src(), "go :rocket: now");
        assert_eq!(w.cursor(), 11);
    }

    #[test]
    fn end_on_soft_row_stops_before_hanging_space() {
        let mut w = we("alpha beta", 0, 8);
        w.end();
        // Row 0 is "alpha " with the trailing space hanging; End parks after
        // the last visible grapheme.
        assert_eq!(w.pos(), (0, 5));
    }
}
