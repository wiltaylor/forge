use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Bar, BarGroup, Widget};

/// Themed vertical bar chart over `(label, value)` pairs, single accent
/// series by default.
#[derive(Clone, Debug)]
pub struct BarChart<'a> {
    data: &'a [(&'a str, u64)],
    color: Option<Color>,
    bar_width: u16,
    theme: Option<&'a Theme>,
}

impl<'a> BarChart<'a> {
    pub fn new(data: &'a [(&'a str, u64)]) -> BarChart<'a> {
        BarChart {
            data,
            color: None,
            bar_width: 7,
            theme: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bar_width(mut self, width: u16) -> Self {
        self.bar_width = width;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for BarChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let color = self.color.unwrap_or(t.accent.base);
        let bars: Vec<Bar> = self
            .data
            .iter()
            .map(|(label, value)| {
                Bar::default()
                    .value(*value)
                    .label((*label).into())
                    .style(Style::new().fg(color))
                    .value_style(Style::new().fg(t.bg[0]).bg(color))
            })
            .collect();
        ratatui::widgets::BarChart::default()
            .data(BarGroup::default().bars(&bars))
            .bar_width(self.bar_width)
            .bar_gap(1)
            .label_style(Style::new().fg(t.fg[2]))
            .render(area, buf);
    }
}
