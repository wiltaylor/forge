//! Presentational toast card — the same look the runtime toaster paints,
//! available standalone for apps that manage their own notification stack.
//! (For fire-and-forget toasts, use `ctx.toast()` from the runtime instead.)

use crate::theme::{Severity, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::Tone;
use egui::Ui;

pub struct Toast<'a> {
    severity: Severity,
    message: &'a str,
}

impl<'a> Toast<'a> {
    pub fn new(severity: Severity, message: &'a str) -> Toast<'a> {
        Toast { severity, message }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let tone = match self.severity {
            Severity::Success => Tone::Success,
            Severity::Warning => Tone::Warning,
            Severity::Danger => Tone::Danger,
            Severity::Info => Tone::Info,
        };
        let (base, _, _) = tone.triple(&t);
        let glyph = match self.severity {
            Severity::Success => Glyph::Check,
            Severity::Warning => Glyph::Warn,
            Severity::Danger => Glyph::Cross,
            Severity::Info => Glyph::Info,
        };
        egui::Frame::new()
            .fill(t.bg[4])
            .stroke(egui::Stroke::new(1.0, t.border.default))
            .corner_radius(egui::CornerRadius::same(t.radius.md as u8))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(glyph.as_str()).color(base));
                    ui.label(egui::RichText::new(self.message).color(t.fg[0]));
                });
            })
            .response
    }
}
