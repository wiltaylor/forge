//! Inline callout: a tinted surface with a solid tone bar, glyph, title, and
//! optional message. Purely presentational — mirrors `@forge/ui`'s `Alert`.

use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::Tone;
use egui::{CornerRadius, Frame, Margin, Rect, Ui};

pub struct Alert<'a> {
    tone: Tone,
    title: &'a str,
    message: Option<&'a str>,
    icon: Option<Glyph>,
}

impl<'a> Alert<'a> {
    pub fn new(tone: Tone, title: &'a str) -> Alert<'a> {
        Alert {
            tone,
            title,
            message: None,
            icon: None,
        }
    }

    pub fn message(mut self, message: &'a str) -> Self {
        self.message = Some(message);
        self
    }

    /// Override the tone's default glyph (✓ / ⚠ / ✗ / ℹ).
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    fn default_glyph(tone: Tone) -> Glyph {
        match tone {
            Tone::Success => Glyph::Check,
            Tone::Warning => Glyph::Warn,
            Tone::Danger => Glyph::Cross,
            Tone::Info | Tone::Accent | Tone::Neutral => Glyph::Info,
        }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let (base, tint, _) = self.tone.triple(&t);
        let glyph = self.icon.unwrap_or(Self::default_glyph(self.tone));

        const BAR: f32 = 3.0;
        let inner = Frame::new()
            .fill(tint)
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin {
                left: (BAR + 11.0) as i8,
                right: 12,
                top: 10,
                bottom: 10,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    ui.label(
                        egui::RichText::new(glyph.as_str())
                            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                            .color(base),
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            egui::RichText::new(self.title)
                                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                                .color(t.fg[0]),
                        );
                        if let Some(message) = self.message {
                            ui.label(
                                egui::RichText::new(message)
                                    .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                                    .color(t.fg[1]),
                            );
                        }
                    });
                });
            });

        // Solid tone bar hugging the left edge, rounded to match the frame.
        let rect = inner.response.rect;
        let bar = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + BAR, rect.max.y));
        ui.painter().rect_filled(
            bar,
            CornerRadius {
                nw: BAR as u8,
                sw: BAR as u8,
                ne: 0,
                se: 0,
            },
            base,
        );

        inner.response
    }
}
