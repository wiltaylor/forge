use crate::theme::{default_theme, series_color, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::symbols;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Widget};

#[derive(Clone, Copy, Debug)]
pub struct LineSeries<'a> {
    pub name: &'a str,
    pub points: &'a [(f64, f64)],
}

impl<'a> LineSeries<'a> {
    pub fn new(name: &'a str, points: &'a [(f64, f64)]) -> LineSeries<'a> {
        LineSeries { name, points }
    }
}

/// Themed multi-series line chart: braille lines in the locked series
/// palette, dim axes. Bounds default to the data extent.
#[derive(Clone, Debug)]
pub struct LineChart<'a> {
    series: &'a [LineSeries<'a>],
    x_bounds: Option<[f64; 2]>,
    y_bounds: Option<[f64; 2]>,
    theme: Option<&'a Theme>,
}

impl<'a> LineChart<'a> {
    pub fn new(series: &'a [LineSeries<'a>]) -> LineChart<'a> {
        LineChart {
            series,
            x_bounds: None,
            y_bounds: None,
            theme: None,
        }
    }

    pub fn x_bounds(mut self, bounds: [f64; 2]) -> Self {
        self.x_bounds = Some(bounds);
        self
    }

    pub fn y_bounds(mut self, bounds: [f64; 2]) -> Self {
        self.y_bounds = Some(bounds);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn extent(&self, pick: impl Fn(&(f64, f64)) -> f64) -> [f64; 2] {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for s in self.series {
            for p in s.points {
                let v = pick(p);
                min = min.min(v);
                max = max.max(v);
            }
        }
        if !min.is_finite() || !max.is_finite() {
            return [0.0, 1.0];
        }
        if (max - min).abs() < f64::EPSILON {
            max = min + 1.0;
        }
        [min, max]
    }
}

impl Widget for LineChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let x = self.x_bounds.unwrap_or_else(|| self.extent(|p| p.0));
        let y = self.y_bounds.unwrap_or_else(|| self.extent(|p| p.1));
        let datasets: Vec<Dataset> = self
            .series
            .iter()
            .enumerate()
            .map(|(i, s)| {
                Dataset::default()
                    .name(s.name)
                    .marker(symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::new().fg(series_color(t, i)))
                    .data(s.points)
            })
            .collect();
        let label = |v: f64| {
            if v.abs() >= 1000.0 {
                format!("{:.1}k", v / 1000.0)
            } else {
                format!("{v:.0}")
            }
        };
        Chart::new(datasets)
            .x_axis(
                Axis::default()
                    .bounds(x)
                    .labels([label(x[0]), label((x[0] + x[1]) / 2.0), label(x[1])])
                    .style(Style::new().fg(t.fg[2])),
            )
            .y_axis(
                Axis::default()
                    .bounds(y)
                    .labels([label(y[0]), label((y[0] + y[1]) / 2.0), label(y[1])])
                    .style(Style::new().fg(t.fg[2])),
            )
            .style(Style::new().bg(t.bg[1]))
            .render(area, buf);
    }
}
