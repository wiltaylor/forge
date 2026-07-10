use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A subtle rule. Horizontal fills the first row of the area; vertical fills
/// the first column.
#[derive(Clone, Debug, Default)]
pub struct Separator<'a> {
    orientation: Orientation,
    theme: Option<&'a Theme>,
}

impl<'a> Separator<'a> {
    pub fn horizontal() -> Separator<'a> {
        Separator { orientation: Orientation::Horizontal, theme: None }
    }

    pub fn vertical() -> Separator<'a> {
        Separator { orientation: Orientation::Vertical, theme: None }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Separator<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let style = Style::new().fg(t.border.subtle);
        match self.orientation {
            Orientation::Horizontal => {
                buf.set_string(area.x, area.y, "─".repeat(area.width as usize), style);
            }
            Orientation::Vertical => {
                for dy in 0..area.height {
                    buf.set_string(area.x, area.y + dy, "│", style);
                }
            }
        }
    }
}
