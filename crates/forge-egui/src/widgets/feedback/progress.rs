//! Linear progress bar — determinate (0..=1) or an indeterminate sweep.

use crate::theme::{FontWeight, Theme};
use crate::widgets::Tone;
use egui::{Rect, Sense, Ui, Vec2, WidgetInfo, WidgetType};

const BAR_HEIGHT: f32 = 4.0;
/// Sweep band width as a fraction of the track (indeterminate mode).
const SWEEP_FRACTION: f32 = 0.3;
/// One full indeterminate sweep, in seconds.
const SWEEP_PERIOD: f64 = 1.2;

pub struct Progress<'a> {
    value: f32,
    tone: Tone,
    label: Option<&'a str>,
    show_value: bool,
    indeterminate: bool,
}

impl<'a> Progress<'a> {
    /// `value` is clamped to `0..=1` (NaN reads as 0).
    pub fn new(value: f32) -> Progress<'a> {
        let value = if value.is_nan() { 0.0 } else { value };
        Progress {
            value: value.clamp(0.0, 1.0),
            tone: Tone::Accent,
            label: None,
            show_value: false,
            indeterminate: false,
        }
    }

    /// The clamped value this bar will paint.
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Right-align the value (`42%`) in the label row.
    pub fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    /// Sweeping-band mode for unknown durations; ignores `value`.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let (base, _, _) = self.tone.triple(&t);
        let width = ui.available_width();

        let inner = ui.vertical(|ui| {
            ui.set_width(width);
            if self.label.is_some() || (self.show_value && !self.indeterminate) {
                ui.horizontal(|ui| {
                    if let Some(label) = self.label {
                        ui.label(
                            egui::RichText::new(label)
                                .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                                .color(t.fg[1]),
                        );
                    }
                    if self.show_value && !self.indeterminate {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}%",
                                    (self.value * 100.0).round() as u32
                                ))
                                .font(t.mono(t.type_scale.sm))
                                .color(t.fg[2]),
                            );
                        });
                    }
                });
                ui.add_space(t.space.x(1.0));
            }

            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(width, BAR_HEIGHT), Sense::hover());
            let value = self.value;
            let indeterminate = self.indeterminate;
            let name = self.label.unwrap_or("progress");
            response.widget_info(move || {
                let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, name);
                if !indeterminate {
                    info.value = Some(f64::from(value));
                }
                info
            });

            if ui.is_rect_visible(rect) {
                let pill = BAR_HEIGHT / 2.0;
                ui.painter().rect_filled(rect, pill, t.bg[3]);
                if self.indeterminate {
                    // A band sweeping left → right, wrapping around.
                    let time = ui.input(|i| i.time);
                    let phase = ((time / SWEEP_PERIOD).fract()) as f32;
                    let band = rect.width() * SWEEP_FRACTION;
                    let start = rect.min.x - band + phase * (rect.width() + band);
                    let sweep = Rect::from_min_max(
                        egui::pos2(start.max(rect.min.x), rect.min.y),
                        egui::pos2((start + band).min(rect.max.x), rect.max.y),
                    );
                    if sweep.width() > 0.0 {
                        ui.painter().rect_filled(sweep, pill, base);
                    }
                    ui.ctx().request_repaint();
                } else if self.value > 0.0 {
                    let fill = Rect::from_min_size(
                        rect.min,
                        Vec2::new((rect.width() * self.value).max(BAR_HEIGHT), BAR_HEIGHT),
                    );
                    ui.painter().rect_filled(fill, pill, base);
                }
            }
            response
        });
        inner.inner
    }
}
