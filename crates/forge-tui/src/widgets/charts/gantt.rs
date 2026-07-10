use crate::text;
use crate::theme::{default_theme, series_color, Severity, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

#[derive(Clone, Copy, Debug)]
pub struct GanttTask<'a> {
    pub label: &'a str,
    pub start: f64,
    pub end: f64,
    /// Semantic status color; falls back to the series palette by row.
    pub severity: Option<Severity>,
}

impl<'a> GanttTask<'a> {
    pub fn new(label: &'a str, start: f64, end: f64) -> GanttTask<'a> {
        GanttTask { label, start, end, severity: None }
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }
}

/// Row-per-task bars on a shared time axis; labels left, min/max time labels
/// on the bottom rule. Values are unitless (timestamps, sprint days, …).
#[derive(Clone, Debug)]
pub struct Gantt<'a> {
    tasks: &'a [GanttTask<'a>],
    bounds: Option<(f64, f64)>,
    theme: Option<&'a Theme>,
}

impl<'a> Gantt<'a> {
    pub fn new(tasks: &'a [GanttTask<'a>]) -> Gantt<'a> {
        Gantt { tasks, bounds: None, theme: None }
    }

    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.bounds = Some((min, max));
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Gantt<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() || self.tasks.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let (min, max) = self.bounds.unwrap_or_else(|| {
            let min = self.tasks.iter().map(|k| k.start).fold(f64::INFINITY, f64::min);
            let max = self.tasks.iter().map(|k| k.end).fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        });
        if !(max - min).is_finite() || max <= min {
            return;
        }
        let label_w = (self
            .tasks
            .iter()
            .map(|k| text::width(k.label))
            .max()
            .unwrap_or(0) as u16
            + 1)
            .min(area.width / 2);
        let track_x = area.x + label_w;
        let track_w = area.width - label_w;
        if track_w < 4 {
            return;
        }
        let scale = |v: f64| -> u16 {
            (((v - min) / (max - min)) * (track_w - 1) as f64).round() as u16
        };
        for (i, task) in self.tasks.iter().enumerate() {
            let y = area.y + i as u16;
            if y + 1 >= area.y + area.height {
                break;
            }
            buf.set_string(
                area.x,
                y,
                text::truncate(task.label, label_w.saturating_sub(1) as usize),
                Style::new().fg(t.fg[1]),
            );
            // Track.
            buf.set_string(
                track_x,
                y,
                "·".repeat(track_w as usize),
                Style::new().fg(t.border.subtle),
            );
            let from = scale(task.start.clamp(min, max));
            let to = scale(task.end.clamp(min, max)).max(from);
            let color = match task.severity {
                Some(s) => t.severity(s).base,
                None => series_color(t, i),
            };
            buf.set_string(
                track_x + from,
                y,
                "█".repeat((to - from + 1) as usize),
                Style::new().fg(color),
            );
        }
        // Axis rule + bounds labels.
        let axis_y = area.y + (self.tasks.len() as u16).min(area.height - 1);
        if axis_y < area.y + area.height {
            buf.set_string(
                track_x,
                axis_y,
                "─".repeat(track_w as usize),
                Style::new().fg(t.border.default),
            );
            let lo = format!("{min:.0}");
            let hi = format!("{max:.0}");
            buf.set_string(track_x, axis_y, &lo, Style::new().fg(t.fg[2]));
            let hw = text::width(&hi) as u16;
            if track_w > hw {
                buf.set_string(track_x + track_w - hw, axis_y, &hi, Style::new().fg(t.fg[2]));
            }
        }
    }
}
