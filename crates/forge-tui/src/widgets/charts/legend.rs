use crate::text;
use crate::theme::{default_theme, series_color, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Shared swatch+label legend row: `■ requests  ■ errors  ■ latency` in the
/// series palette order (matching whichever chart it sits beside).
#[derive(Clone, Debug)]
pub struct Legend<'a> {
    labels: &'a [&'a str],
    theme: Option<&'a Theme>,
}

impl<'a> Legend<'a> {
    pub fn new(labels: &'a [&'a str]) -> Legend<'a> {
        Legend { labels, theme: None }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Legend<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let right = area.x + area.width;
        let mut x = area.x;
        for (i, label) in self.labels.iter().enumerate() {
            let need = 2 + text::width(label) as u16 + 2;
            if x + need > right {
                break;
            }
            buf.set_string(x, area.y, "■", Style::new().fg(series_color(t, i)));
            buf.set_string(x + 2, area.y, *label, Style::new().fg(t.fg[1]));
            x += need;
        }
    }
}
