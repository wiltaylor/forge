//! Multi-series line chart: 1.5pt lines in the locked palette, optional area
//! fill at 15 % series alpha, nice-tick grid, nearest-point hover tooltip.
//! X is the point index (forge-tui parity); pass `.x_labels(..)` for
//! category labels under the axis.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{color::with_alpha, series_color, Theme};
use crate::widgets::charts::{self, nice_ticks, TipRow};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{Pos2, Sense, Shape, Stroke, Ui, Vec2};

/// A named series; `points[i]` is plotted at x = i.
#[derive(Clone, Debug)]
pub struct LineSeries {
    pub name: String,
    pub points: Vec<f64>,
}

impl LineSeries {
    pub fn new(name: impl Into<String>, points: impl Into<Vec<f64>>) -> LineSeries {
        LineSeries {
            name: name.into(),
            points: points.into(),
        }
    }
}

pub struct LineChart<'a> {
    series: &'a [LineSeries],
    height: f32,
    fill: bool,
    x_labels: Option<&'a [&'a str]>,
}

impl<'a> LineChart<'a> {
    pub fn new(series: &'a [LineSeries]) -> LineChart<'a> {
        LineChart {
            series,
            height: 160.0,
            fill: false,
            x_labels: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Fill the area under each series at 15 % of the series color.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Category labels for integer x positions `0..n`.
    pub fn x_labels(mut self, labels: &'a [&'a str]) -> Self {
        self.x_labels = Some(labels);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let width = ui.available_width().max(120.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height), Sense::hover());
        let mut outcome = Outcome::Ignored;

        let n = self
            .series
            .iter()
            .map(|s| s.points.len())
            .max()
            .unwrap_or(0);
        if ui.is_rect_visible(rect) && n > 0 {
            let data_min = self
                .series
                .iter()
                .flat_map(|s| s.points.iter().cloned())
                .fold(f64::INFINITY, f64::min)
                .min(0.0);
            let data_max = self
                .series
                .iter()
                .flat_map(|s| s.points.iter().cloned())
                .fold(f64::NEG_INFINITY, f64::max);
            let ticks = nice_ticks(data_min, data_max, 4);
            let (y_lo, y_hi) = (
                ticks.first().copied().unwrap_or(0.0),
                ticks.last().copied().unwrap_or(1.0),
            );
            let span = (y_hi - y_lo).max(f64::MIN_POSITIVE);

            let plot = charts::plot_rect(rect);
            let y_of = |v: f64| plot.max.y - (((v - y_lo) / span) as f32) * plot.height();
            let x_of = |i: usize| plot.min.x + plot.width() * (i as f32) / ((n - 1).max(1) as f32);
            charts::y_axis(ui, &t, plot, &ticks, y_of);

            // X labels: categories when given, sparse mono indices otherwise.
            let paint_x = |i: usize, text: &str, mono: bool| {
                let font = if mono {
                    t.mono(t.type_scale.xs)
                } else {
                    t.font(ui.ctx(), crate::theme::FontWeight::Regular, t.type_scale.xs)
                };
                let g = ui.painter().layout_no_wrap(text.to_owned(), font, t.fg[2]);
                ui.painter().galley(
                    egui::pos2(x_of(i) - g.size().x / 2.0, rect.max.y - g.size().y - 3.0),
                    g,
                    t.fg[2],
                );
            };
            if let Some(labels) = self.x_labels {
                let step = (labels.len() * 40 / plot.width().max(1.0) as usize).max(1);
                for (i, label) in labels.iter().enumerate().take(n).step_by(step) {
                    paint_x(i, label, false);
                }
            } else {
                let mut marks = vec![0, n / 4, n / 2, 3 * n / 4, n - 1];
                marks.dedup();
                for i in marks {
                    paint_x(i, &i.to_string(), true);
                }
            }

            // Area fills first (under every line), then strokes.
            if self.fill {
                for (si, series) in self.series.iter().enumerate() {
                    if series.points.len() < 2 {
                        continue;
                    }
                    let color = with_alpha(series_color(&t, si), 38); // ≈ 15 %
                    let base = y_of(y_lo.max(0.0).min(y_hi));
                    let mut mesh = Mesh::default();
                    for (i, v) in series.points.iter().enumerate() {
                        mesh.vertices.push(Vertex {
                            pos: Pos2::new(x_of(i), y_of(*v)),
                            uv: WHITE_UV,
                            color,
                        });
                        mesh.vertices.push(Vertex {
                            pos: Pos2::new(x_of(i), base),
                            uv: WHITE_UV,
                            color,
                        });
                        if i > 0 {
                            let k = (2 * i) as u32;
                            mesh.add_triangle(k - 2, k - 1, k);
                            mesh.add_triangle(k - 1, k + 1, k);
                        }
                    }
                    ui.painter().add(Shape::mesh(mesh));
                }
            }
            for (si, series) in self.series.iter().enumerate() {
                let pts: Vec<Pos2> = series
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, v)| Pos2::new(x_of(i), y_of(*v)))
                    .collect();
                if pts.len() >= 2 {
                    ui.painter()
                        .add(Shape::line(pts, Stroke::new(1.5, series_color(&t, si))));
                } else if let Some(p) = pts.first() {
                    ui.painter().circle_filled(*p, 2.0, series_color(&t, si));
                }
            }

            // Nearest-point hover marker + tooltip.
            if let Some(pointer) = response.hover_pos() {
                let mut best: Option<(usize, usize, Pos2, f32)> = None;
                for (si, series) in self.series.iter().enumerate() {
                    for (i, v) in series.points.iter().enumerate() {
                        let p = Pos2::new(x_of(i), y_of(*v));
                        let d = p.distance(pointer);
                        if best.is_none_or(|(_, _, _, bd)| d < bd) {
                            best = Some((si, i, p, d));
                        }
                    }
                }
                if let Some((si, i, p, d)) = best {
                    if d <= 24.0 {
                        outcome = Outcome::Consumed;
                        let color = series_color(&t, si);
                        ui.painter().circle_filled(p, 3.5, color);
                        ui.painter()
                            .circle_stroke(p, 3.5, Stroke::new(1.5, t.bg[1]));
                        let title = self
                            .x_labels
                            .and_then(|l| l.get(i).copied())
                            .map(String::from)
                            .unwrap_or_else(|| format!("#{i}"));
                        charts::tooltip(
                            ui,
                            response.id.with("tip"),
                            Some(&title),
                            &[TipRow {
                                swatch: Some(color),
                                text: format!(
                                    "{}: {}",
                                    self.series[si].name,
                                    charts::fmt(self.series[si].points[i])
                                ),
                            }],
                        );
                    }
                }
            }
        }

        ForgeResponse::new(response, outcome)
    }
}
