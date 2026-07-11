//! Indeterminate activity spinner: a rotating accent arc, time-driven.

use crate::theme::{FontWeight, Theme};
use egui::{Sense, Ui, Vec2, WidgetInfo, WidgetType};

pub struct Spinner<'a> {
    size: f32,
    label: Option<&'a str>,
}

impl<'a> Spinner<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Spinner<'a> {
        Spinner {
            size: 16.0,
            label: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let inner = ui.horizontal(|ui| {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
            let name = self.label.unwrap_or("loading");
            response.widget_info(move || {
                WidgetInfo::labeled(WidgetType::ProgressIndicator, true, name)
            });

            if ui.is_rect_visible(rect) {
                let time = ui.input(|i| i.time);
                let start = (time * std::f64::consts::TAU / 0.9) as f32;
                let radius = self.size / 2.0 - 1.5;
                let stroke_w = (self.size / 8.0).max(1.5);
                // A 270° arc from the rotating start angle.
                let n = 24;
                let points: Vec<egui::Pos2> = (0..=n)
                    .map(|i| {
                        let a = start + (i as f32 / n as f32) * std::f32::consts::TAU * 0.75;
                        rect.center() + Vec2::new(a.cos(), a.sin()) * radius
                    })
                    .collect();
                ui.painter().add(egui::Shape::line(
                    points,
                    egui::Stroke::new(stroke_w, t.accent.base),
                ));
                ui.ctx().request_repaint();
            }

            if let Some(label) = self.label {
                ui.label(
                    egui::RichText::new(label)
                        .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                        .color(t.fg[1]),
                );
            }
            response
        });
        inner.inner
    }
}
