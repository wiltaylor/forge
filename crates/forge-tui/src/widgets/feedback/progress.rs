use crate::text;
use crate::theme::{default_theme, Severity, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Determinate progress bar (one row): optional label, sub-cell-precision
/// fill, optional percentage readout.
#[derive(Clone, Debug)]
pub struct Progress<'a> {
    ratio: f64,
    label: Option<&'a str>,
    severity: Option<Severity>,
    show_percent: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Progress<'a> {
    pub fn new(ratio: f64) -> Progress<'a> {
        Progress {
            ratio: ratio.clamp(0.0, 1.0),
            label: None,
            severity: None,
            show_percent: true,
            theme: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Color the fill semantically instead of accent.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn show_percent(mut self, show: bool) -> Self {
        self.show_percent = show;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

const EIGHTHS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];

impl Widget for Progress<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let fill_color: Color = match self.severity {
            Some(s) => t.severity(s).base,
            None => t.accent.base,
        };
        let mut x = area.x;
        let mut w = area.width;
        if let Some(label) = self.label {
            let label = text::truncate(label, (w / 3) as usize);
            let lw = text::width(&label) as u16 + 1;
            buf.set_string(x, area.y, label, Style::new().fg(t.fg[1]));
            x += lw;
            w = w.saturating_sub(lw);
        }
        let mut pct_w = 0;
        if self.show_percent {
            pct_w = 5; // " 100%"
            w = w.saturating_sub(pct_w);
        }
        if w > 0 {
            let cells = w as f64 * self.ratio;
            let full = cells.floor() as u16;
            let frac = ((cells - full as f64) * 8.0).round() as usize;
            let bar_style = Style::new().fg(fill_color).bg(t.bg[3]);
            let track_style = Style::new().fg(t.bg[3]).bg(t.bg[3]);
            buf.set_string(x, area.y, "█".repeat(full as usize), bar_style);
            let mut end = x + full;
            if frac > 0 && full < w {
                buf.set_string(end, area.y, EIGHTHS[frac.min(7)], bar_style);
                end += 1;
            }
            if end < x + w {
                buf.set_string(end, area.y, " ".repeat((x + w - end) as usize), track_style);
            }
        }
        if self.show_percent && pct_w > 0 {
            let pct = format!("{:>4}%", (self.ratio * 100.0).round() as u16);
            buf.set_string(x + w, area.y, pct, Style::new().fg(t.fg[2]));
        }
    }
}
