use forge_tui::prelude::*;
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme) {
    if area.height < 6 {
        return;
    }
    let blocks = BlockGrid::new(2).gap(2).split(
        area,
        &[
            BlockSpec::new(1, 10),
            BlockSpec::new(1, 10),
            BlockSpec::new(1, 9),
            BlockSpec::new(1, 9),
            BlockSpec::new(2, 2),
        ],
    );

    // Line chart.
    if let Some(r) = blocks.first().filter(|r| r.height > 2) {
        frame.render_widget(
            Eyebrow::new("LineChart").theme(t),
            Rect::new(r.x, r.y, r.width, 1),
        );
        let requests: Vec<(f64, f64)> = (0..24)
            .map(|h| (h as f64, 40.0 + 30.0 * ((h as f64) / 3.0).sin() + h as f64))
            .collect();
        let errors: Vec<(f64, f64)> = (0..24)
            .map(|h| (h as f64, 8.0 + 6.0 * ((h as f64) / 5.0).cos()))
            .collect();
        let series = [
            LineSeries::new("requests", &requests),
            LineSeries::new("errors", &errors),
        ];
        frame.render_widget(
            LineChart::new(&series).theme(t),
            Rect::new(r.x, r.y + 1, r.width, r.height - 1),
        );
    }

    // Pie.
    if let Some(r) = blocks.get(1).filter(|r| r.height > 2) {
        frame.render_widget(
            Eyebrow::new("PieChart").theme(t),
            Rect::new(r.x, r.y, r.width, 1),
        );
        let slices = [
            PieSlice::new("api", 46.0),
            PieSlice::new("static", 28.0),
            PieSlice::new("events", 15.0),
            PieSlice::new("auth", 8.0),
            PieSlice::new("other", 3.0),
        ];
        frame.render_widget(
            PieChart::new(&slices).theme(t),
            Rect::new(r.x, r.y + 1, r.width, r.height - 1),
        );
    }

    // Bar chart.
    if let Some(r) = blocks.get(2).filter(|r| r.height > 2) {
        frame.render_widget(
            Eyebrow::new("BarChart").theme(t),
            Rect::new(r.x, r.y, r.width, 1),
        );
        let data = [
            ("mon", 32u64),
            ("tue", 41),
            ("wed", 28),
            ("thu", 52),
            ("fri", 47),
        ];
        frame.render_widget(
            BarChart::new(&data).theme(t),
            Rect::new(r.x, r.y + 1, r.width, r.height - 1),
        );
    }

    // Gantt.
    if let Some(r) = blocks.get(3).filter(|r| r.height > 2) {
        frame.render_widget(
            Eyebrow::new("Gantt").theme(t),
            Rect::new(r.x, r.y, r.width, 1),
        );
        let tasks = [
            GanttTask::new("design", 0.0, 3.0),
            GanttTask::new("theme", 2.0, 5.0),
            GanttTask::new("widgets", 4.0, 11.0),
            GanttTask::new("gallery", 5.0, 12.0),
            GanttTask::new("docs", 11.0, 14.0).severity(Severity::Warning),
        ];
        frame.render_widget(
            Gantt::new(&tasks).theme(t),
            Rect::new(r.x, r.y + 1, r.width, r.height - 1),
        );
    }

    // Sparkline + legend strip.
    if let Some(r) = blocks.get(4).filter(|r| r.height >= 2) {
        let data: Vec<u64> = (0..r.width as u64)
            .map(|i| 3 + (i * 7 % 13) + ctx.frame % 5)
            .collect();
        frame.render_widget(
            forge_tui::widgets::charts::Sparkline::new(&data).theme(t),
            Rect::new(r.x, r.y, r.width / 2, 1),
        );
        frame.render_widget(
            Legend::new(&["requests", "errors", "success", "queued", "info"]).theme(t),
            Rect::new(r.x, r.y + 1, r.width, 1),
        );
    }
}
