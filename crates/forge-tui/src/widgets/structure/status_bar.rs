use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// One-row bottom bar: dim hints on the left, right-aligned meta on the
/// right, on a raised surface.
#[derive(Clone, Debug, Default)]
pub struct StatusBar<'a> {
    left: &'a str,
    right: &'a str,
    theme: Option<&'a Theme>,
}

impl<'a> StatusBar<'a> {
    pub fn new(left: &'a str) -> StatusBar<'a> {
        StatusBar { left, right: "", theme: None }
    }

    pub fn right(mut self, right: &'a str) -> Self {
        self.right = right;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let style = Style::new().fg(t.fg[2]).bg(t.bg[1]);
        buf.set_style(Rect::new(area.x, area.y, area.width, 1), style);
        buf.set_string(
            area.x + 1,
            area.y,
            text::truncate(self.left, area.width.saturating_sub(2) as usize),
            style,
        );
        let rw = text::width(self.right) as u16;
        if rw > 0 && area.width > rw + 2 {
            buf.set_string(area.x + area.width - rw - 1, area.y, self.right, style);
        }
    }
}
