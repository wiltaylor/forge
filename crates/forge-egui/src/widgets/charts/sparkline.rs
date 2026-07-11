//! Tiny inline trend line: `fg[2]` 1.5pt stroke + a toned endpoint dot.
//! No axes, no tooltip — web `Sparkline` parity (default 96×28).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::Theme;
use crate::widgets::Tone;
use egui::{Pos2, Sense, Shape, Stroke, Ui, Vec2};

pub struct Sparkline<'a> {
    points: &'a [f64],
    size: Vec2,
    tone: Tone,
}

impl<'a> Sparkline<'a> {
    pub fn new(points: &'a [f64]) -> Sparkline<'a> {
        Sparkline {
            points,
            size: Vec2::new(96.0, 28.0),
            tone: Tone::Accent,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Vec2::new(width, height);
        self
    }

    /// Tone of the endpoint dot (default accent).
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::hover());

        if ui.is_rect_visible(rect) && !self.points.is_empty() {
            let min = self.points.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = self
                .points
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let span = (max - min).max(f64::MIN_POSITIVE);
            let n = self.points.len();
            let pts: Vec<Pos2> = self
                .points
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    Pos2::new(
                        rect.min.x
                            + 2.0
                            + (i as f32 / (n - 1).max(1) as f32) * (rect.width() - 4.0),
                        rect.max.y - 3.0 - (((v - min) / span) as f32) * (rect.height() - 6.0),
                    )
                })
                .collect();
            let last = *pts.last().expect("non-empty");
            if pts.len() >= 2 {
                ui.painter()
                    .add(Shape::line(pts, Stroke::new(1.5, t.fg[2])));
            }
            let (dot, _, _) = self.tone.triple(&t);
            ui.painter().circle_filled(last, 2.5, dot);
        }

        ForgeResponse::new(response, Outcome::Ignored)
    }
}
