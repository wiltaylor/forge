use crate::text;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Uppercase, dim section label — the small tracked caption above headings.
#[derive(Clone, Debug)]
pub struct Eyebrow<'a> {
    label: &'a str,
}

impl<'a> Eyebrow<'a> {
    pub fn new(label: &'a str) -> Eyebrow<'a> {
        Eyebrow { label }
    }
}

impl Widget for Eyebrow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            let label = self.label.to_uppercase();
            buf.set_string(
                area.x,
                area.y,
                text::truncate(&label, area.width as usize),
                Style::new().fg(t.fg[2]),
            );
        });
    }
}
