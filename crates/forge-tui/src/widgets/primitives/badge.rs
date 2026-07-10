use crate::text;
use crate::theme::{default_theme, Severity, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Inline status pill: ` label ` on a semantic tint (neutral by default).
#[derive(Clone, Debug)]
pub struct Badge<'a> {
    label: &'a str,
    severity: Option<Severity>,
    theme: Option<&'a Theme>,
}

impl<'a> Badge<'a> {
    pub fn new(label: &'a str) -> Badge<'a> {
        Badge { label, severity: None, theme: None }
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Natural width in cells, for layout math.
    pub fn width(&self) -> u16 {
        (text::width(self.label) as u16).saturating_add(2)
    }
}

impl Widget for Badge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let style = match self.severity {
            Some(s) => {
                let tri = t.severity(s);
                Style::new().fg(tri.fg).bg(tri.bg)
            }
            None => Style::new().fg(t.fg[1]).bg(t.bg[3]),
        };
        let label = text::truncate(self.label, area.width.saturating_sub(2) as usize);
        let w = (text::width(&label) as u16 + 2).min(area.width);
        buf.set_style(Rect::new(area.x, area.y, w, 1), style);
        buf.set_string(area.x + 1, area.y, label, style);
    }
}
