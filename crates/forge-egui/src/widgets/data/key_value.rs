//! Dense definition list: dim keys in a fixed right-padded column, bright
//! values (optionally monospace).

use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use egui::{Sense, Ui, Vec2};

pub struct KeyValue<'a> {
    pairs: &'a [(&'a str, &'a str)],
    mono: bool,
}

impl<'a> KeyValue<'a> {
    pub fn new(pairs: &'a [(&'a str, &'a str)]) -> KeyValue<'a> {
        KeyValue { pairs, mono: false }
    }

    /// Render values in the monospace family (ids, hashes, paths).
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let key_font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        let value_font = if self.mono {
            t.mono(t.type_scale.sm)
        } else {
            t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base)
        };
        let row_h = t.type_scale.base + 8.0;
        let pad = t.space.x(4.0);

        // Fixed key column: widest key + padding.
        let key_w = self
            .pairs
            .iter()
            .map(|(k, _)| util::galley(ui, *k, key_font.clone(), t.fg[2]).size().x)
            .fold(0.0f32, f32::max)
            + pad;

        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), row_h * self.pairs.len() as f32),
            Sense::hover(),
        );
        if ui.is_rect_visible(rect) {
            for (i, (k, v)) in self.pairs.iter().enumerate() {
                let cy = rect.min.y + row_h * (i as f32 + 0.5);
                let g = util::galley(ui, *k, key_font.clone(), t.fg[2]);
                ui.painter()
                    .galley(egui::pos2(rect.min.x, cy - g.size().y / 2.0), g, t.fg[2]);
                let g = util::galley(ui, *v, value_font.clone(), t.fg[0]);
                let clip = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
                clip.galley(
                    egui::pos2(rect.min.x + key_w, cy - g.size().y / 2.0),
                    g,
                    t.fg[0],
                );
            }
        }
        response
    }
}
