//! KPI tile: eyebrow label, large value, optional trend delta.

use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::Tone;
use egui::{CornerRadius, Frame, Margin, Stroke, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

pub struct Stat<'a> {
    label: &'a str,
    value: &'a str,
    delta: Option<(&'a str, Trend, Tone)>,
}

impl<'a> Stat<'a> {
    pub fn new(label: &'a str, value: &'a str) -> Stat<'a> {
        Stat {
            label,
            value,
            delta: None,
        }
    }

    pub fn delta(mut self, text: &'a str, trend: Trend, tone: Tone) -> Self {
        self.delta = Some((text, trend, tone));
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.lg as u8))
            .inner_margin(Margin::same(t.space.x(4.0) as i8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                crate::widgets::Eyebrow::new(self.label).show(ui);
                ui.add_space(t.space.x(1.0));
                ui.label(
                    egui::RichText::new(self.value)
                        .font(t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.h2))
                        .color(t.fg[0]),
                );
                if let Some((text, trend, tone)) = self.delta {
                    let (base, _, _) = tone.triple(&t);
                    let arrow = match trend {
                        Trend::Up => Glyph::ArrowUp.as_str(),
                        Trend::Down => Glyph::ArrowDown.as_str(),
                        Trend::Flat => "→",
                    };
                    ui.label(
                        egui::RichText::new(format!("{arrow} {text}"))
                            .size(t.type_scale.sm)
                            .color(base),
                    );
                }
            })
            .response
    }
}
