//! Donut/pie chart. Slices take the locked palette in order; anything past
//! the fifth slice folds into a single `fg[2]` "Other" slice (the palette
//! contract — never cycle). Hover expands a slice by 2pt and shows a
//! percentage tooltip; a swatch legend renders at the right.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{series_color, FontWeight, Theme};
use crate::widgets::charts::{self, TipRow};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{Color32, Pos2, Sense, Shape, Stroke, Ui, Vec2};
use std::f32::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Debug)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

impl PieSlice {
    pub fn new(label: impl Into<String>, value: f64) -> PieSlice {
        PieSlice {
            label: label.into(),
            value,
        }
    }
}

/// Fold slices past the palette into one "Other" bucket. Returns
/// `(label, value)` pairs — at most [`CHART_SERIES_LEN`] + 1 entries, where a
/// sixth entry is always the "Other" sum.
///
/// [`CHART_SERIES_LEN`]: crate::theme::CHART_SERIES_LEN
pub(crate) fn fold_slices(slices: &[PieSlice]) -> Vec<(String, f64)> {
    let keep = crate::theme::CHART_SERIES_LEN;
    let mut out: Vec<(String, f64)> = slices
        .iter()
        .take(keep)
        .map(|s| (s.label.clone(), s.value.max(0.0)))
        .collect();
    if slices.len() > keep {
        let other: f64 = slices[keep..].iter().map(|s| s.value.max(0.0)).sum();
        out.push(("Other".to_owned(), other));
    }
    out
}

pub struct PieChart<'a> {
    slices: &'a [PieSlice],
    height: f32,
    donut: bool,
    center: Option<&'a str>,
    legend: bool,
}

impl<'a> PieChart<'a> {
    pub fn new(slices: &'a [PieSlice]) -> PieChart<'a> {
        PieChart {
            slices,
            height: 160.0,
            donut: true,
            center: None,
            legend: true,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Donut (default) vs solid pie.
    pub fn donut(mut self, donut: bool) -> Self {
        self.donut = donut;
        self
    }

    /// Text in the donut hole.
    pub fn center(mut self, center: &'a str) -> Self {
        self.center = Some(center);
        self
    }

    /// Swatch + label + percentage rows at the right (default on).
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let width = ui.available_width().max(120.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height), Sense::hover());
        let mut outcome = Outcome::Ignored;

        let folded = fold_slices(self.slices);
        let total: f64 = folded.iter().map(|(_, v)| v).sum();
        if ui.is_rect_visible(rect) && total > 0.0 {
            let radius = (self.height / 2.0 - 10.0).max(20.0);
            let center = Pos2::new(rect.min.x + radius + 12.0, rect.center().y);
            let inner_r = if self.donut { radius * 0.6 } else { 0.0 };

            // Which slice is hovered? (pointer in the annulus + angle range)
            let hovered = response.hover_pos().and_then(|p| {
                let v = p - center;
                let r = v.length();
                if r < inner_r || r > radius + 2.0 {
                    return None;
                }
                // Angle in the slice convention: 0 at 12 o'clock, clockwise.
                let ang = (v.y.atan2(v.x) + FRAC_PI_2).rem_euclid(TAU);
                let mut start = 0.0f32;
                for (i, (_, value)) in folded.iter().enumerate() {
                    let sweep = (value / total) as f32 * TAU;
                    if ang >= start && ang < start + sweep {
                        return Some(i);
                    }
                    start += sweep;
                }
                None
            });

            let mut boundaries: Vec<f32> = Vec::with_capacity(folded.len());
            let mut start = -FRAC_PI_2;
            for (i, (_, value)) in folded.iter().enumerate() {
                let sweep = (value / total) as f32 * TAU;
                boundaries.push(start);
                let color = series_color(&t, i);
                let outer = if hovered == Some(i) {
                    radius + 2.0
                } else {
                    radius
                };
                slice_mesh(ui, center, inner_r, outer, start, sweep, color);
                start += sweep;
            }
            // Slice separators in the surface color (web strokes bg-1).
            if folded.len() > 1 {
                for a in boundaries {
                    let dir = Vec2::new(a.cos(), a.sin());
                    ui.painter().line_segment(
                        [center + dir * inner_r, center + dir * (radius + 2.0)],
                        Stroke::new(2.0, t.bg[1]),
                    );
                }
            }

            if self.donut {
                if let Some(text) = self.center {
                    let g = ui.painter().layout_no_wrap(
                        text.to_owned(),
                        t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.base),
                        t.fg[0],
                    );
                    ui.painter().galley(center - g.size() / 2.0, g, t.fg[0]);
                }
            }

            if self.legend {
                let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm);
                let mono = t.mono(t.type_scale.sm);
                let row_h = t.type_scale.sm + 8.0;
                let x = rect.min.x + radius * 2.0 + 32.0;
                let pct_col = folded
                    .iter()
                    .map(|(label, _)| {
                        ui.painter()
                            .layout_no_wrap(label.clone(), font.clone(), t.fg[1])
                            .size()
                            .x
                    })
                    .fold(0.0f32, f32::max)
                    + 24.0;
                let mut y = rect.center().y - row_h * folded.len() as f32 / 2.0;
                for (i, (label, value)) in folded.iter().enumerate() {
                    let cy = y + row_h / 2.0;
                    ui.painter()
                        .circle_filled(Pos2::new(x + 4.0, cy), 4.0, series_color(&t, i));
                    let g = ui
                        .painter()
                        .layout_no_wrap(label.clone(), font.clone(), t.fg[1]);
                    ui.painter()
                        .galley(Pos2::new(x + 14.0, cy - g.size().y / 2.0), g, t.fg[1]);
                    let pct = format!("{:>2.0} %", value / total * 100.0);
                    let g = ui.painter().layout_no_wrap(pct, mono.clone(), t.fg[2]);
                    ui.painter().galley(
                        Pos2::new(x + 14.0 + pct_col, cy - g.size().y / 2.0),
                        g,
                        t.fg[2],
                    );
                    y += row_h;
                }
            }

            if let Some(i) = hovered {
                outcome = Outcome::Consumed;
                let (label, value) = &folded[i];
                charts::tooltip(
                    ui,
                    response.id.with("tip"),
                    Some(label),
                    &[TipRow {
                        swatch: Some(series_color(&t, i)),
                        text: format!("{} ({:.0} %)", charts::fmt(*value), value / total * 100.0),
                    }],
                );
            }
        }

        ForgeResponse::new(response, outcome)
    }
}

/// Tessellate one annular slice as a triangle strip (≥ 24 segments so arcs
/// stay smooth at gallery sizes).
fn slice_mesh(
    ui: &Ui,
    center: Pos2,
    inner_r: f32,
    outer_r: f32,
    start: f32,
    sweep: f32,
    color: Color32,
) {
    if sweep <= 0.0 {
        return;
    }
    let steps = ((sweep / TAU * 128.0).ceil() as usize).clamp(24, 256);
    let mut mesh = Mesh::default();
    for s in 0..=steps {
        let a = start + sweep * s as f32 / steps as f32;
        let dir = Vec2::new(a.cos(), a.sin());
        mesh.vertices.push(Vertex {
            pos: center + dir * outer_r,
            uv: WHITE_UV,
            color,
        });
        mesh.vertices.push(Vertex {
            pos: center + dir * inner_r.max(0.0),
            uv: WHITE_UV,
            color,
        });
        if s > 0 {
            let k = (2 * s) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    ui.painter().add(Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::{fold_slices, PieSlice};

    #[test]
    fn six_slices_fold_into_other() {
        let slices: Vec<PieSlice> = (1..=6)
            .map(|i| PieSlice::new(format!("s{i}"), i as f64))
            .collect();
        let folded = fold_slices(&slices);
        assert_eq!(folded.len(), 6);
        assert_eq!(folded[4], ("s5".to_owned(), 5.0));
        assert_eq!(folded[5], ("Other".to_owned(), 6.0));
    }

    #[test]
    fn many_slices_sum_the_remainder() {
        let slices: Vec<PieSlice> = (1..=8)
            .map(|i| PieSlice::new(format!("s{i}"), 1.0))
            .collect();
        let folded = fold_slices(&slices);
        assert_eq!(folded.len(), 6);
        assert_eq!(folded[5], ("Other".to_owned(), 3.0));
    }

    #[test]
    fn five_or_fewer_slices_do_not_fold() {
        let slices: Vec<PieSlice> = (1..=5)
            .map(|i| PieSlice::new(format!("s{i}"), 1.0))
            .collect();
        assert_eq!(fold_slices(&slices).len(), 5);
        assert!(fold_slices(&slices).iter().all(|(l, _)| l != "Other"));
    }
}
