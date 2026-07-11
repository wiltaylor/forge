//! Status indicator dot, optionally pulsing.

use crate::theme::Theme;
use crate::widgets::Tone;
use egui::{Sense, Ui, Vec2};

pub struct StatusDot {
    tone: Tone,
    pulse: bool,
}

impl StatusDot {
    pub fn new(tone: Tone) -> StatusDot {
        StatusDot { tone, pulse: false }
    }

    /// Animate an expanding fading ring (live/health indicators).
    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let (base, _, _) = self.tone.triple(&t);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
        if ui.is_rect_visible(rect) {
            let center = rect.center();
            ui.painter().circle_filled(center, 4.0, base);
            if self.pulse {
                let cycle = 1.6;
                let phase = (ui.input(|i| i.time) % cycle / cycle) as f32;
                let r = 4.0 + phase * 5.0;
                let alpha = ((1.0 - phase) * 120.0) as u8;
                ui.painter().circle_stroke(
                    center,
                    r,
                    egui::Stroke::new(1.5, crate::theme::color::with_alpha(base, alpha)),
                );
                ui.ctx().request_repaint();
            }
        }
        response
    }
}
