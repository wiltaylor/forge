use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Uppercase, dim section label — the small tracked caption above headings.
#[derive(Clone, Debug)]
pub struct Eyebrow<'a> {
    label: &'a str,
    theme: Option<&'a Theme>,
}

impl<'a> Eyebrow<'a> {
    pub fn new(label: &'a str) -> Eyebrow<'a> {
        Eyebrow { label, theme: None }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Eyebrow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let label = self.label.to_uppercase();
        buf.set_string(
            area.x,
            area.y,
            text::truncate(&label, area.width as usize),
            Style::new().fg(t.fg[2]),
        );
    }
}
