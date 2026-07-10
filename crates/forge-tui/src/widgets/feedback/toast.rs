use crate::text;
use crate::theme::{default_theme, Severity, Theme};
use crate::widgets::primitives::Glyph;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Widget};

/// A single toast box — normally painted by the runtime `Toaster`, exposed
/// for manual composition. Three rows tall: border, message, border.
#[derive(Clone, Debug)]
pub struct ToastView<'a> {
    severity: Severity,
    message: &'a str,
    theme: Option<&'a Theme>,
}

impl<'a> ToastView<'a> {
    pub fn new(severity: Severity, message: &'a str) -> ToastView<'a> {
        ToastView { severity, message, theme: None }
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

    /// Preferred size at a maximum width.
    pub fn size(&self, max_width: u16) -> (u16, u16) {
        let w = (text::width(self.message) as u16 + 6).min(max_width).max(8);
        (w, 3)
    }
}

impl Widget for ToastView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let tri = t.severity(self.severity);
        let block = Block::bordered()
            .border_style(Style::new().fg(tri.base).bg(t.bg[4]))
            .style(Style::new().bg(t.bg[4]));
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.is_empty() {
            return;
        }
        buf.set_string(inner.x + 1, inner.y, self.glyph().as_str(), Style::new().fg(tri.base).bg(t.bg[4]));
        buf.set_string(
            inner.x + 3,
            inner.y,
            text::truncate(self.message, inner.width.saturating_sub(4) as usize),
            Style::new().fg(t.fg[0]).bg(t.bg[4]),
        );
    }
}
