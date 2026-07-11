//! Key-cap chip.

use crate::theme::Theme;
use crate::widgets::util;
use egui::{CornerRadius, Sense, Stroke, Ui, Vec2};

pub struct Kbd<'a> {
    keys: &'a str,
}

impl<'a> Kbd<'a> {
    pub fn new(keys: &'a str) -> Kbd<'a> {
        Kbd { keys }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let g = util::galley(ui, self.keys, t.mono(t.type_scale.xs), t.fg[1]);
        let size = Vec2::new(g.size().x + 10.0, 18.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            let radius = CornerRadius::same(t.radius.sm as u8);
            ui.painter().rect_filled(rect, radius, t.bg[3]);
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(1.0, t.border.default),
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .galley(rect.center() - g.size() / 2.0, g, t.fg[1]);
        }
        response
    }
}
