//! Inline status pill.

use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use crate::widgets::Tone;
use egui::{CornerRadius, Sense, Ui, Vec2};

pub struct Badge<'a> {
    label: &'a str,
    tone: Tone,
    dot: bool,
}

impl<'a> Badge<'a> {
    pub fn new(label: &'a str) -> Badge<'a> {
        Badge {
            label,
            tone: Tone::Neutral,
            dot: false,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Lead with a small solid dot in the tone color.
    pub fn dot(mut self, dot: bool) -> Self {
        self.dot = dot;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let (base, bg, fg) = self.tone.triple(&t);
        let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        let g = util::galley(ui, self.label, font, fg);

        let height = 20.0;
        let pad_x = 8.0;
        let dot_w = if self.dot { 10.0 } else { 0.0 };
        let width = g.size().x + dot_w + pad_x * 2.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, CornerRadius::same((height / 2.0) as u8), bg);
            let mut x = rect.min.x + pad_x;
            if self.dot {
                ui.painter()
                    .circle_filled(egui::pos2(x + 2.0, rect.center().y), 3.0, base);
                x += dot_w;
            }
            ui.painter()
                .galley(egui::pos2(x, rect.center().y - g.size().y / 2.0), g, fg);
        }
        response
    }
}
