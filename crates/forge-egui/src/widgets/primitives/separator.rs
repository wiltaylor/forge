//! Horizontal / vertical rule.

use crate::theme::Theme;
use egui::{Sense, Ui, Vec2};

pub struct Separator {
    vertical: bool,
    spacing: f32,
}

impl Separator {
    pub fn new() -> Separator {
        Separator {
            vertical: false,
            spacing: 8.0,
        }
    }

    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Blank space on each side of the line.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let size = if self.vertical {
            Vec2::new(self.spacing * 2.0 + 1.0, ui.available_height().max(8.0))
        } else {
            Vec2::new(ui.available_width().max(8.0), self.spacing * 2.0 + 1.0)
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if self.vertical {
                let x = rect.center().x;
                painter.line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(1.0, t.border.default),
                );
            } else {
                let y = rect.center().y;
                painter.line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(1.0, t.border.default),
                );
            }
        }
        response
    }
}

impl Default for Separator {
    fn default() -> Self {
        Separator::new()
    }
}
