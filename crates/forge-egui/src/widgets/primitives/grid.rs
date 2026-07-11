//! Simple equal-column grid layout helper (the web `fgrid` analog). Cells
//! are collected then laid out in rows of `cols` equal-width columns.

use egui::Ui;

type Cell<'c> = Box<dyn FnOnce(&mut Ui) + 'c>;

pub struct Grid {
    cols: usize,
    gap: f32,
}

pub struct GridCells<'c> {
    cells: Vec<Cell<'c>>,
}

impl<'c> GridCells<'c> {
    pub fn cell(&mut self, f: impl FnOnce(&mut Ui) + 'c) {
        self.cells.push(Box::new(f));
    }
}

impl Grid {
    pub fn new(cols: usize) -> Grid {
        Grid {
            cols: cols.max(1),
            gap: 12.0,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn show(self, ui: &mut Ui, collect: impl FnOnce(&mut GridCells)) {
        let mut cells = GridCells { cells: Vec::new() };
        collect(&mut cells);

        let col_w = ((ui.available_width() - self.gap * (self.cols as f32 - 1.0))
            / self.cols as f32)
            .max(40.0);
        let mut iter = cells.cells.into_iter().peekable();
        while iter.peek().is_some() {
            let row: Vec<Cell> = iter.by_ref().take(self.cols).collect();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = self.gap;
                for cell in row {
                    ui.allocate_ui_with_layout(
                        egui::vec2(col_w, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(col_w);
                            cell(ui);
                        },
                    );
                }
            });
            ui.add_space(self.gap);
        }
    }
}
