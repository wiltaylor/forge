//! Zero-dep charts on the locked CVD palette. Colors come from
//! [`theme::series_color`](crate::theme::series_color) — `[accent, danger,
//! success, warning, info]`, with everything past the fifth series folded
//! into a `fg[2]` "Other" bucket (never cycled). Geometry mirrors
//! `packages/charts/src/charts.tsx`; every chart fills the available width,
//! takes `.height(..)`, and shows a Forge-framed tooltip on hover.

mod bar;
mod gantt;
mod legend;
mod line;
mod pie;
mod sparkline;
mod ticks;

pub use bar::{BarChart, BarGroup};
pub use gantt::{Gantt, GanttTask};
pub use legend::Legend;
pub use line::{LineChart, LineSeries};
pub use pie::{PieChart, PieSlice};
pub use sparkline::Sparkline;
pub use ticks::nice_ticks;

use crate::theme::{FontWeight, Theme};
use egui::{Color32, CornerRadius, Margin, Rect, Sense, Stroke, Ui, Vec2};

/// Plot padding shared by the axis charts — mirrors the web `PAD` constant.
const PAD_L: f32 = 44.0;
const PAD_R: f32 = 12.0;
const PAD_T: f32 = 8.0;
const PAD_B: f32 = 22.0;

/// Compact number formatting — parity with the web `fmt` helper
/// (`1.2 M` / `3.4 k` / trimmed 2-decimal).
fn fmt(n: f64) -> String {
    let (v, suffix, digits) = if n.abs() >= 1e6 {
        (n / 1e6, " M", 1)
    } else if n.abs() >= 1e3 {
        (n / 1e3, " k", 1)
    } else {
        (n, "", 2)
    };
    let s = format!("{v:.digits$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        &s
    };
    format!("{s}{suffix}")
}

/// The inner plot rect (chart rect minus axis padding).
fn plot_rect(rect: Rect) -> Rect {
    Rect::from_min_max(
        egui::pos2(rect.min.x + PAD_L, rect.min.y + PAD_T),
        egui::pos2(rect.max.x - PAD_R, rect.max.y - PAD_B),
    )
}

/// Horizontal gridlines + right-aligned mono tick labels for the y axis.
/// The zero line gets `border.default`; the rest stay `border.subtle`.
fn y_axis(ui: &Ui, t: &Theme, plot: Rect, ticks: &[f64], y_of: impl Fn(f64) -> f32) {
    let font = t.mono(t.type_scale.xs);
    for &tick in ticks {
        let y = y_of(tick);
        let color = if tick == 0.0 {
            t.border.default
        } else {
            t.border.subtle
        };
        ui.painter().line_segment(
            [egui::pos2(plot.min.x, y), egui::pos2(plot.max.x, y)],
            Stroke::new(1.0, color),
        );
        let g = ui
            .painter()
            .layout_no_wrap(fmt(tick), font.clone(), t.fg[2]);
        ui.painter().galley(
            egui::pos2(plot.min.x - 6.0 - g.size().x, y - g.size().y / 2.0),
            g,
            t.fg[2],
        );
    }
}

/// A tooltip row: optional series swatch + text.
struct TipRow {
    pub swatch: Option<Color32>,
    pub text: String,
}

/// Forge-framed hover tooltip at the pointer: `bg[4]` surface, 1pt border,
/// sm text. `title` renders Medium `fg[0]`; rows render `fg[1]`.
fn tooltip(ui: &Ui, id: egui::Id, title: Option<&str>, rows: &[TipRow]) {
    let ctx = ui.ctx();
    let Some(pos) = ctx.pointer_hover_pos() else {
        return;
    };
    let t = Theme::of(ctx);
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .fixed_pos(pos + Vec2::new(14.0, 14.0))
        .interactable(false)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(t.bg[4])
                .stroke(Stroke::new(1.0, t.border.default))
                .corner_radius(CornerRadius::same(t.radius.md as u8))
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    if let Some(title) = title {
                        ui.label(
                            egui::RichText::new(title)
                                .font(t.font(ctx, FontWeight::Medium, t.type_scale.sm))
                                .color(t.fg[0]),
                        );
                    }
                    for row in rows {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if let Some(color) = row.swatch {
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 3.5, color);
                            }
                            ui.label(
                                egui::RichText::new(&row.text)
                                    .font(t.font(ctx, FontWeight::Regular, t.type_scale.sm))
                                    .color(t.fg[1]),
                            );
                        });
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::fmt;

    #[test]
    fn fmt_matches_web_helper() {
        assert_eq!(fmt(0.0), "0");
        assert_eq!(fmt(42.0), "42");
        assert_eq!(fmt(2.5), "2.5");
        assert_eq!(fmt(1500.0), "1.5 k");
        assert_eq!(fmt(2_000_000.0), "2 M");
        assert_eq!(fmt(-1200.0), "-1.2 k");
    }
}
