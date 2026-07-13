use crate::text;
use crate::theme::{default_theme, Severity, Theme};
use crate::widgets::primitives::Glyph;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

/// Inline banner: semantic tint fill, solid left bar, icon + title + wrapped
/// body. Give it 1 row for title-only, more for the body.
#[derive(Clone, Debug)]
pub struct Alert<'a> {
    severity: Severity,
    title: &'a str,
    body: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Alert<'a> {
    pub fn new(severity: Severity, title: &'a str) -> Alert<'a> {
        Alert {
            severity,
            title,
            body: None,
            theme: None,
        }
    }

    pub fn body(mut self, body: &'a str) -> Self {
        self.body = Some(body);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn glyph(&self) -> Glyph {
        match self.severity {
            Severity::Success => Glyph::Check,
            Severity::Warning => Glyph::Warn,
            Severity::Danger => Glyph::Cross,
            Severity::Info => Glyph::Info,
        }
    }

    /// Rows needed at `width` cells (title + wrapped body).
    pub fn height(&self, width: u16) -> u16 {
        let body_w = width.saturating_sub(4);
        1 + self
            .body
            .map(|b| text::wrap(b, body_w.max(1) as usize).len() as u16)
            .unwrap_or(0)
    }
}

impl Widget for Alert<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let tri = t.severity(self.severity);
        buf.set_style(area, Style::new().bg(tri.bg));
        for dy in 0..area.height {
            buf.set_string(
                area.x,
                area.y + dy,
                "▎",
                Style::new().fg(tri.base).bg(tri.bg),
            );
        }
        let inner_w = area.width.saturating_sub(4) as usize;
        let title = format!("{} {}", self.glyph().as_str(), self.title);
        buf.set_string(
            area.x + 2,
            area.y,
            text::truncate(&title, inner_w + 2),
            Style::new()
                .fg(tri.fg)
                .bg(tri.bg)
                .add_modifier(Modifier::BOLD),
        );
        if let Some(body) = self.body {
            let style = Style::new().fg(t.fg[1]).bg(tri.bg);
            for (i, line) in text::wrap(body, inner_w.max(1)).into_iter().enumerate() {
                let y = area.y + 1 + i as u16;
                if y >= area.y + area.height {
                    break;
                }
                buf.set_string(area.x + 4, y, line, style);
            }
        }
    }
}
