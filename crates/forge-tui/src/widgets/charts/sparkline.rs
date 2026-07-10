use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Themed wrapper over ratatui's sparkline: accent series on the card
/// surface.
#[derive(Clone, Debug)]
pub struct Sparkline<'a> {
    data: &'a [u64],
    color: Option<Color>,
    theme: Option<&'a Theme>,
}

impl<'a> Sparkline<'a> {
    pub fn new(data: &'a [u64]) -> Sparkline<'a> {
        Sparkline { data, color: None, theme: None }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Sparkline<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        ratatui::widgets::Sparkline::default()
            .data(self.data)
            .style(Style::new().fg(self.color.unwrap_or(t.accent.base)))
            .render(area, buf);
    }
}
