use ratatui::layout::Rect;

/// One dashboard block: how many grid columns it spans and how many rows
/// tall it is.
#[derive(Clone, Copy, Debug)]
pub struct BlockSpec {
    pub span: u16,
    pub rows: u16,
}

impl BlockSpec {
    pub fn new(span: u16, rows: u16) -> BlockSpec {
        BlockSpec { span, rows }
    }
}

/// Dashboard tile layout: a fixed column grid where each block spans 1..=N
/// columns, flowing left-to-right and wrapping. Rows take the height of
/// their tallest block. Returns one Rect per spec (zero-height when clipped
/// below the area), render your widgets into them.
#[derive(Clone, Copy, Debug)]
pub struct BlockGrid {
    cols: u16,
    gap: u16,
}

impl BlockGrid {
    pub fn new(cols: u16) -> BlockGrid {
        BlockGrid {
            cols: cols.max(1),
            gap: 1,
        }
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn split(&self, area: Rect, blocks: &[BlockSpec]) -> Vec<Rect> {
        let col_w = (area.width.saturating_sub(self.gap * (self.cols - 1))) / self.cols;
        let mut out = Vec::with_capacity(blocks.len());
        let mut cursor_col = 0u16;
        let mut y = area.y;
        let mut row_h = 0u16;
        let bottom = area.y + area.height;
        for spec in blocks {
            let span = spec.span.clamp(1, self.cols);
            if cursor_col + span > self.cols {
                // wrap
                y = y.saturating_add(row_h + self.gap);
                cursor_col = 0;
                row_h = 0;
            }
            let x = area.x + cursor_col * (col_w + self.gap);
            let w = col_w * span + self.gap * (span - 1);
            let h = if y >= bottom {
                0
            } else {
                spec.rows.min(bottom - y)
            };
            out.push(Rect::new(x, y, w, h));
            cursor_col += span;
            row_h = row_h.max(spec.rows);
        }
        out
    }
}
