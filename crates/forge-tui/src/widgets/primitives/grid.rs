use ratatui::layout::Rect;

/// Cell-grid layout helper: splits an area into equal columns (fixed count or
/// auto-fit by minimum column width) with a uniform gap, flowing row-major.
///
/// ```
/// # use forge_tui::widgets::Grid;
/// # use ratatui::layout::Rect;
/// let cells = Grid::auto(24).gap(1).cells(Rect::new(0, 0, 80, 12), 6, 4);
/// assert_eq!(cells.len(), 6);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    cols: Option<u16>,
    min_col_width: u16,
    gap: u16,
}

impl Grid {
    /// A fixed number of columns.
    pub fn new(cols: u16) -> Grid {
        Grid { cols: Some(cols.max(1)), min_col_width: 1, gap: 1 }
    }

    /// As many columns as fit at `min_col_width` cells each (responsive).
    pub fn auto(min_col_width: u16) -> Grid {
        Grid { cols: None, min_col_width: min_col_width.max(1), gap: 1 }
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Lay out `count` cells of `row_height` rows each. Cells that fall below
    /// the area are clipped away (zero-height), so callers can render
    /// unconditionally.
    pub fn cells(&self, area: Rect, count: usize, row_height: u16) -> Vec<Rect> {
        let cols = match self.cols {
            Some(c) => c,
            None => ((area.width + self.gap) / (self.min_col_width + self.gap)).max(1),
        } as usize;
        let cols = cols.max(1);
        let col_w = (area.width.saturating_sub(self.gap * (cols as u16 - 1))) / cols as u16;
        (0..count)
            .map(|i| {
                let col = (i % cols) as u16;
                let row = (i / cols) as u16;
                let x = area.x + col * (col_w + self.gap);
                let y = area.y.saturating_add(row * (row_height + self.gap));
                let bottom = area.y + area.height;
                let h = if y >= bottom {
                    0
                } else {
                    row_height.min(bottom - y)
                };
                Rect::new(x, y, col_w, h)
            })
            .collect()
    }
}
