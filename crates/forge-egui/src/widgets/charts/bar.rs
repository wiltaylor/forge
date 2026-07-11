//! Grouped/stacked vertical bar chart — geometry mirrors the web `BarChart`
//! (band layout, 24pt max bar width, rounded bar tops).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{series_color, shift, FontWeight, Theme};
use crate::widgets::charts::{self, nice_ticks, TipRow};
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

/// One x-axis category: a label plus one value per series.
#[derive(Clone, Debug)]
pub struct BarGroup {
    pub label: String,
    pub values: Vec<f64>,
}

impl BarGroup {
    pub fn new(label: impl Into<String>, values: impl Into<Vec<f64>>) -> BarGroup {
        BarGroup {
            label: label.into(),
            values: values.into(),
        }
    }
}

/// Grouped vertical bars on the locked series palette, with nice-tick
/// gridlines and hover tooltips. `.stacked(true)` stacks series instead.
pub struct BarChart<'a> {
    groups: &'a [BarGroup],
    names: Option<&'a [&'a str]>,
    height: f32,
    stacked: bool,
}

impl<'a> BarChart<'a> {
    pub fn new(groups: &'a [BarGroup]) -> BarChart<'a> {
        BarChart {
            groups,
            names: None,
            height: 160.0,
            stacked: false,
        }
    }

    /// Series names for tooltips (defaults to `Series N`).
    pub fn names(mut self, names: &'a [&'a str]) -> Self {
        self.names = Some(names);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Stack the series in each group instead of grouping them side by side.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let width = ui.available_width().max(120.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height), Sense::hover());
        let mut outcome = Outcome::Ignored;

        if ui.is_rect_visible(rect) && !self.groups.is_empty() {
            let series_n = self
                .groups
                .iter()
                .map(|g| g.values.len())
                .max()
                .unwrap_or(0);
            let max_total = self
                .groups
                .iter()
                .map(|g| {
                    if self.stacked {
                        g.values.iter().map(|v| v.max(0.0)).sum::<f64>()
                    } else {
                        g.values.iter().cloned().fold(0.0f64, f64::max)
                    }
                })
                .fold(0.0f64, f64::max);
            let ticks = nice_ticks(0.0, max_total, 4);
            let y_max = ticks.last().copied().unwrap_or(1.0).max(f64::MIN_POSITIVE);

            let plot = charts::plot_rect(rect);
            let y_of = |v: f64| plot.max.y - ((v / y_max) as f32) * plot.height();
            charts::y_axis(ui, &t, plot, &ticks, y_of);

            let band = plot.width() / self.groups.len() as f32;
            let group_n = if self.stacked { 1 } else { series_n.max(1) };
            let bar_w = (band * 0.7 / group_n as f32).min(24.0);
            let hover = response.hover_pos();
            let mut tip: Option<(usize, usize, Rect)> = None;

            // X labels under each band + bar fills.
            let label_font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs);
            for (ci, group) in self.groups.iter().enumerate() {
                let cx = plot.min.x + band * ci as f32 + band / 2.0;
                let g =
                    ui.painter()
                        .layout_no_wrap(group.label.clone(), label_font.clone(), t.fg[2]);
                ui.painter().galley(
                    egui::pos2(cx - g.size().x / 2.0, rect.max.y - g.size().y - 3.0),
                    g,
                    t.fg[2],
                );

                if self.stacked {
                    let mut acc = 0.0f64;
                    for (si, v) in group.values.iter().enumerate() {
                        let v = v.max(0.0);
                        let (y0, y1) = (y_of(acc), y_of(acc + v));
                        acc += v;
                        let bar = Rect::from_min_max(
                            egui::pos2(cx - bar_w / 2.0, y1),
                            egui::pos2(cx + bar_w / 2.0, y0),
                        );
                        if bar.height() <= 0.0 {
                            continue;
                        }
                        let hovered = hover.is_some_and(|p| bar.contains(p));
                        if hovered {
                            tip = Some((ci, si, bar));
                        }
                        let color = series_color(&t, si);
                        let fill = if hovered { shift(color, 0.10) } else { color };
                        ui.painter().rect_filled(bar, CornerRadius::ZERO, fill);
                        // Hairline seam so stacked segments read apart.
                        ui.painter().rect_stroke(
                            bar,
                            CornerRadius::ZERO,
                            Stroke::new(1.0, t.bg[1]),
                            StrokeKind::Inside,
                        );
                    }
                } else {
                    for (si, v) in group.values.iter().enumerate() {
                        let v = v.max(0.0);
                        let x = cx - (bar_w * group_n as f32) / 2.0 + bar_w * si as f32;
                        let bar = Rect::from_min_max(
                            egui::pos2(x + 0.5, y_of(v)),
                            egui::pos2(x + bar_w - 0.5, y_of(0.0)),
                        );
                        if bar.height() <= 0.0 {
                            continue;
                        }
                        let hovered = hover.is_some_and(|p| bar.contains(p));
                        if hovered {
                            tip = Some((ci, si, bar));
                        }
                        let color = series_color(&t, si);
                        let fill = if hovered { shift(color, 0.10) } else { color };
                        let r = 4.0f32.min(bar.height() / 2.0).min(bar.width() / 2.0) as u8;
                        ui.painter().rect_filled(
                            bar,
                            CornerRadius {
                                nw: r,
                                ne: r,
                                sw: 0,
                                se: 0,
                            },
                            fill,
                        );
                    }
                }
            }

            if let Some((ci, si, bar)) = tip {
                outcome = Outcome::Consumed;
                ui.painter().rect_stroke(
                    bar,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, t.border.strong),
                    StrokeKind::Outside,
                );
                let group = &self.groups[ci];
                let name = self
                    .names
                    .and_then(|n| n.get(si).copied())
                    .map(String::from)
                    .unwrap_or_else(|| format!("Series {}", si + 1));
                charts::tooltip(
                    ui,
                    response.id.with("tip"),
                    Some(&group.label),
                    &[TipRow {
                        swatch: Some(series_color(&t, si)),
                        text: format!("{name}: {}", charts::fmt(group.values[si])),
                    }],
                );
            }
        }

        ForgeResponse::new(response, outcome)
    }
}
