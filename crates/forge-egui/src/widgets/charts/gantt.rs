//! Gantt rows on a unitless f64 time axis (the caller maps dates to numbers,
//! forge-tui parity). Task bars are a 15 % series tint with the done fraction
//! painted solid; `.marker(..)` draws an accent dashed "today" line.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{color::with_alpha, series_color, FontWeight, Theme};
use crate::widgets::charts::{self, nice_ticks, TipRow};
use egui::{CornerRadius, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

#[derive(Clone, Debug)]
pub struct GanttTask {
    pub label: String,
    pub start: f64,
    pub end: f64,
    /// Palette index for the bar color (folds to "Other" past the fifth).
    pub series: usize,
    /// Completed fraction `0.0..=1.0`, painted solid over the tint.
    pub done: Option<f32>,
}

impl GanttTask {
    pub fn new(label: impl Into<String>, start: f64, end: f64) -> GanttTask {
        GanttTask {
            label: label.into(),
            start,
            end,
            series: 0,
            done: None,
        }
    }

    pub fn series(mut self, series: usize) -> Self {
        self.series = series;
        self
    }

    pub fn done(mut self, done: f32) -> Self {
        self.done = Some(done);
        self
    }
}

pub struct Gantt<'a> {
    tasks: &'a [GanttTask],
    height: f32,
    marker: Option<f64>,
}

const AXIS_H: f32 = 18.0;

impl<'a> Gantt<'a> {
    pub fn new(tasks: &'a [GanttTask]) -> Gantt<'a> {
        Gantt {
            tasks,
            height: 160.0,
            marker: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Accent dashed vertical line at this axis position (today line).
    pub fn marker(mut self, at: f64) -> Self {
        self.marker = Some(at);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let width = ui.available_width().max(160.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height), Sense::hover());
        let mut outcome = Outcome::Ignored;

        if ui.is_rect_visible(rect) && !self.tasks.is_empty() {
            let min = self
                .tasks
                .iter()
                .map(|k| k.start)
                .fold(f64::INFINITY, f64::min);
            let max = self
                .tasks
                .iter()
                .map(|k| k.end)
                .fold(f64::NEG_INFINITY, f64::max);
            if !min.is_finite() || !max.is_finite() || max <= min {
                return ForgeResponse::new(response, outcome);
            }

            let label_font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm);
            let label_w = self
                .tasks
                .iter()
                .map(|k| {
                    ui.painter()
                        .layout_no_wrap(k.label.clone(), label_font.clone(), t.fg[1])
                        .size()
                        .x
                })
                .fold(0.0f32, f32::max)
                .min(rect.width() * 0.35)
                + 16.0;

            let track = Rect::from_min_max(
                Pos2::new(rect.min.x + label_w, rect.min.y + AXIS_H),
                Pos2::new(rect.max.x - charts::PAD_R, rect.max.y - 4.0),
            );

            // Time axis: nice ticks anchor the domain so gridlines land on
            // round values; labels sit on the top axis row.
            let ticks = nice_ticks(min, max, 5);
            let (t0, t1) = (
                ticks.first().copied().unwrap_or(min),
                ticks.last().copied().unwrap_or(max),
            );
            let span = (t1 - t0).max(f64::MIN_POSITIVE);
            let x_of = |v: f64| track.min.x + (((v - t0) / span) as f32) * track.width();
            let mono = t.mono(t.type_scale.xs);
            for &tick in &ticks {
                let x = x_of(tick);
                ui.painter().line_segment(
                    [Pos2::new(x, track.min.y), Pos2::new(x, track.max.y)],
                    Stroke::new(1.0, t.border.subtle),
                );
                let g = ui
                    .painter()
                    .layout_no_wrap(charts::fmt(tick), mono.clone(), t.fg[2]);
                ui.painter().galley(
                    Pos2::new(
                        (x - g.size().x / 2.0).clamp(track.min.x, track.max.x - g.size().x),
                        rect.min.y + (AXIS_H - g.size().y) / 2.0,
                    ),
                    g,
                    t.fg[2],
                );
            }

            let row_h = track.height() / self.tasks.len() as f32;
            let bar_h = (row_h - 8.0).clamp(6.0, 16.0);
            let hover = response.hover_pos();
            let mut tip: Option<usize> = None;

            for (i, task) in self.tasks.iter().enumerate() {
                let y = track.min.y + row_h * i as f32;
                let row =
                    Rect::from_min_max(Pos2::new(rect.min.x, y), Pos2::new(track.max.x, y + row_h));
                let hovered = hover.is_some_and(|p| row.contains(p));
                if hovered {
                    tip = Some(i);
                    ui.painter()
                        .rect_filled(row, CornerRadius::same(t.radius.sm as u8), t.bg[2]);
                }

                // Left label column (clipped to its width).
                let g =
                    ui.painter()
                        .layout_no_wrap(task.label.clone(), label_font.clone(), t.fg[1]);
                ui.painter()
                    .with_clip_rect(Rect::from_min_max(
                        Pos2::new(rect.min.x, y),
                        Pos2::new(rect.min.x + label_w - 8.0, y + row_h),
                    ))
                    .galley(
                        Pos2::new(rect.min.x, y + (row_h - g.size().y) / 2.0),
                        g,
                        t.fg[1],
                    );

                let color = series_color(&t, task.series);
                let x0 = x_of(task.start.clamp(t0, t1));
                let x1 = x_of(task.end.clamp(t0, t1)).max(x0 + 2.0);
                let bar = Rect::from_min_max(
                    Pos2::new(x0, y + (row_h - bar_h) / 2.0),
                    Pos2::new(x1, y + (row_h + bar_h) / 2.0),
                );
                let radius = CornerRadius::same(t.radius.sm as u8);
                ui.painter().rect_filled(bar, radius, with_alpha(color, 56));
                if let Some(done) = task.done {
                    let done = done.clamp(0.0, 1.0);
                    if done > 0.0 {
                        let solid = Rect::from_min_max(
                            bar.min,
                            Pos2::new(bar.min.x + bar.width() * done, bar.max.y),
                        );
                        ui.painter().rect_filled(solid, radius, color);
                    }
                } else {
                    // No progress semantics — paint the whole bar solid.
                    ui.painter().rect_filled(bar, radius, color);
                }
            }

            if let Some(at) = self.marker {
                if at >= t0 && at <= t1 {
                    let x = x_of(at);
                    ui.painter().add(Shape::dashed_line(
                        &[Pos2::new(x, track.min.y), Pos2::new(x, track.max.y)],
                        Stroke::new(1.0, t.accent.base),
                        4.0,
                        3.0,
                    ));
                }
            }

            if let Some(i) = tip {
                outcome = Outcome::Consumed;
                let task = &self.tasks[i];
                let mut rows = vec![TipRow {
                    swatch: Some(series_color(&t, task.series)),
                    text: format!("{} – {}", charts::fmt(task.start), charts::fmt(task.end)),
                }];
                if let Some(done) = task.done {
                    rows.push(TipRow {
                        swatch: None,
                        text: format!("{:.0} % done", done.clamp(0.0, 1.0) * 100.0),
                    });
                }
                charts::tooltip(ui, response.id.with("tip"), Some(&task.label), &rows);
            }
        }

        ForgeResponse::new(response, outcome)
    }
}
