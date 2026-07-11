//! Charts: the zero-dep viz kit on the locked CVD series palette.

use forge_egui::prelude::*;

const SERIES_NAMES: &[&str] = &["requests", "errors", "latency"];

pub fn draw(ui: &mut egui::Ui) {
    ui.columns(2, |cols| {
        Card::new().title("Bar (grouped)").show(&mut cols[0], |ui| {
            let groups = [
                BarGroup::new("mon", [42.0, 12.0, 30.0]),
                BarGroup::new("tue", [58.0, 9.0, 26.0]),
                BarGroup::new("wed", [51.0, 17.0, 34.0]),
                BarGroup::new("thu", [74.0, 6.0, 22.0]),
                BarGroup::new("fri", [66.0, 14.0, 28.0]),
            ];
            let _ = BarChart::new(&groups)
                .names(SERIES_NAMES)
                .height(170.0)
                .show(ui);
            ui.add_space(8.0);
            let _ = Legend::new(SERIES_NAMES).show(ui);
        });
        Card::new().title("Bar (stacked)").show(&mut cols[1], |ui| {
            let groups = [
                BarGroup::new("q1", [30.0, 22.0, 12.0]),
                BarGroup::new("q2", [38.0, 18.0, 16.0]),
                BarGroup::new("q3", [44.0, 25.0, 10.0]),
                BarGroup::new("q4", [52.0, 20.0, 18.0]),
            ];
            let _ = BarChart::new(&groups)
                .names(SERIES_NAMES)
                .stacked(true)
                .height(170.0)
                .show(ui);
            ui.add_space(8.0);
            let _ = Legend::new(SERIES_NAMES).show(ui);
        });
    });
    ui.add_space(12.0);

    Card::new().title("Line (2 series + fill)").show(ui, |ui| {
        let series = [
            LineSeries::new(
                "p50",
                [
                    12.0, 14.0, 11.0, 18.0, 24.0, 22.0, 30.0, 27.0, 34.0, 31.0, 38.0, 42.0,
                ],
            ),
            LineSeries::new(
                "p95",
                [
                    22.0, 26.0, 21.0, 34.0, 41.0, 38.0, 52.0, 47.0, 58.0, 54.0, 63.0, 71.0,
                ],
            ),
        ];
        let _ = LineChart::new(&series)
            .fill(true)
            .height(170.0)
            .x_labels(&[
                "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
            ])
            .show(ui);
        ui.add_space(8.0);
        let _ = Legend::new(&["p50", "p95"]).show(ui);
    });
    ui.add_space(12.0);

    ui.columns(2, |cols| {
        Card::new()
            .title("Donut (6 slices → Other fold)")
            .show(&mut cols[0], |ui| {
                let slices = [
                    PieSlice::new("gateway", 38.0),
                    PieSlice::new("auth", 24.0),
                    PieSlice::new("billing", 17.0),
                    PieSlice::new("search", 11.0),
                    PieSlice::new("ingest", 7.0),
                    PieSlice::new("metrics", 3.0),
                ];
                let _ = PieChart::new(&slices)
                    .center("100 GB")
                    .height(170.0)
                    .show(ui);
            });
        Card::new().title("Sparklines").show(&mut cols[1], |ui| {
            let t = Theme::of(ui.ctx());
            let rows: [(&str, &[f64], Tone); 3] = [
                (
                    "throughput",
                    &[3.0, 5.0, 4.0, 8.0, 7.0, 11.0, 9.0, 14.0, 13.0, 17.0],
                    Tone::Accent,
                ),
                (
                    "errors",
                    &[2.0, 1.0, 4.0, 3.0, 7.0, 5.0, 9.0, 8.0, 12.0, 15.0],
                    Tone::Danger,
                ),
                (
                    "uptime",
                    &[9.0, 9.5, 9.2, 9.8, 9.6, 9.9, 9.7, 9.9, 10.0, 10.0],
                    Tone::Success,
                ),
            ];
            for (label, points, tone) in rows {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [90.0, 28.0],
                        egui::Label::new(
                            egui::RichText::new(label)
                                .size(t.type_scale.sm)
                                .color(t.fg[1]),
                        ),
                    );
                    let _ = Sparkline::new(points).size(140.0, 28.0).tone(tone).show(ui);
                });
            }
        });
    });
    ui.add_space(12.0);

    Card::new().title("Gantt (marker = today)").show(ui, |ui| {
        let tasks = [
            GanttTask::new("design tokens", 0.0, 6.0)
                .series(0)
                .done(1.0),
            GanttTask::new("primitives", 4.0, 12.0).series(1).done(0.85),
            GanttTask::new("forms", 10.0, 18.0).series(2).done(0.6),
            GanttTask::new("charts", 15.0, 24.0).series(3).done(0.3),
            GanttTask::new("calendar", 20.0, 27.0).series(4).done(0.1),
            GanttTask::new("release", 26.0, 30.0).series(5),
        ];
        let _ = Gantt::new(&tasks).marker(16.0).height(180.0).show(ui);
    });
    ui.add_space(12.0);

    Card::new().title("Legend (palette order)").show(ui, |ui| {
        let _ = Legend::new(&["accent", "danger", "success", "warning", "info", "other"])
            .wrap(true)
            .show(ui);
    });
}
