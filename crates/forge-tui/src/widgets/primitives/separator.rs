use crate::widgets::paint;
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
pub struct Separator {
    orientation: Orientation,
}

impl Separator {
    pub fn horizontal() -> Separator {
        Separator {
            orientation: Orientation::Horizontal,
        }
    }

    pub fn vertical() -> Separator {
        Separator {
            orientation: Orientation::Vertical,
        }
    }
}

impl Widget for Separator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
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
        });
    }
}
