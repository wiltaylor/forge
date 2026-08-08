use crate::text;
use crate::theme::TextRole;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

/// Page title with optional description below (2 rows) — pair with `Crumbs`
/// above and action buttons rendered by the caller on the title row's right.
#[derive(Clone, Debug)]
pub struct PageHead<'a> {
    title: &'a str,
    description: Option<&'a str>,
}

impl<'a> PageHead<'a> {
    pub fn new(title: &'a str) -> PageHead<'a> {
        PageHead {
            title,
            description: None,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }
}

impl Widget for PageHead<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            buf.set_string(
                area.x,
                area.y,
                text::truncate(self.title, area.width as usize),
                Style::new()
                    .fg(t.text(TextRole::Primary))
                    .add_modifier(Modifier::BOLD),
            );
            if let (Some(desc), true) = (self.description, area.height >= 2) {
                buf.set_string(
                    area.x,
                    area.y + 1,
                    text::truncate(desc, area.width as usize),
                    Style::new().fg(t.text(TextRole::Tertiary)),
                );
            }
        });
    }
}
