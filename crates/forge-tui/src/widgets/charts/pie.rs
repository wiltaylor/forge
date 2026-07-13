use crate::text;
use crate::theme::{default_theme, series_color, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::symbols;
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::Widget;

#[derive(Clone, Copy, Debug)]
pub struct PieSlice<'a> {
    pub label: &'a str,
    pub value: f64,
}

impl<'a> PieSlice<'a> {
    pub fn new(label: &'a str, value: f64) -> PieSlice<'a> {
        PieSlice { label, value }
    }
}

/// Braille donut chart with a legend at the right. Slices take the locked
/// series palette in order; slices past the fifth fold into "Other".
#[derive(Clone, Debug)]
pub struct PieChart<'a> {
    slices: &'a [PieSlice<'a>],
    donut: bool,
    legend: bool,
    theme: Option<&'a Theme>,
}

impl<'a> PieChart<'a> {
    pub fn new(slices: &'a [PieSlice<'a>]) -> PieChart<'a> {
        PieChart {
            slices,
            donut: true,
            legend: true,
            theme: None,
        }
    }

    pub fn donut(mut self, donut: bool) -> Self {
        self.donut = donut;
        self
    }

    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for PieChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() || self.slices.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let total: f64 = self.slices.iter().map(|s| s.value.max(0.0)).sum();
        if total <= 0.0 {
            return;
        }
        let legend_w = if self.legend {
            (self
                .slices
                .iter()
                .map(|s| text::width(s.label))
                .max()
                .unwrap_or(0) as u16
                + 9)
            .min(area.width / 2)
        } else {
            0
        };
        let chart = Rect::new(area.x, area.y, area.width - legend_w, area.height);
        let inner_r = if self.donut { 0.55 } else { 0.0 };

        let canvas = Canvas::default()
            .marker(symbols::Marker::Braille)
            .x_bounds([-1.15, 1.15])
            .y_bounds([-1.15, 1.15])
            .paint(|ctx| {
                let mut start = -std::f64::consts::FRAC_PI_2; // 12 o'clock
                for (i, slice) in self.slices.iter().enumerate() {
                    let frac = slice.value.max(0.0) / total;
                    let sweep = frac * std::f64::consts::TAU;
                    let color = series_color(t, i);
                    let mut points = Vec::new();
                    // Sample the annulus of this slice.
                    let steps = (sweep * 120.0).ceil().max(2.0) as usize;
                    for a in 0..=steps {
                        let ang = start + sweep * a as f64 / steps as f64;
                        let mut r = inner_r;
                        while r <= 1.0 {
                            points.push((ang.cos(), ang.sin() * -1.0, r));
                            r += 0.04;
                        }
                    }
                    let coords: Vec<(f64, f64)> =
                        points.iter().map(|(cx, cy, r)| (cx * r, cy * r)).collect();
                    ctx.draw(&Points {
                        coords: &coords,
                        color,
                    });
                    start += sweep;
                }
            });
        canvas.render(chart, buf);

        if self.legend && legend_w > 0 {
            let lx = area.x + area.width - legend_w;
            for (i, slice) in self.slices.iter().enumerate() {
                let y = area.y + i as u16;
                if y >= area.y + area.height {
                    break;
                }
                let pct = format!("{:>3.0}%", slice.value.max(0.0) / total * 100.0);
                buf.set_string(lx, y, "■", Style::new().fg(series_color(t, i)));
                buf.set_string(
                    lx + 2,
                    y,
                    text::truncate(slice.label, legend_w.saturating_sub(8) as usize),
                    Style::new().fg(t.fg[1]),
                );
                buf.set_string(lx + legend_w - 5, y, pct, Style::new().fg(t.fg[2]));
            }
        }
    }
}
