use crate::text;
use crate::theme::{Surface, TextRole};
use crate::widgets::overlays::place;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Clear, Widget};

/// One-line hint chip anchored to a rect. Terminals have no hover, so show
/// it while the anchored widget is focused (or on a help key).
#[derive(Clone, Debug)]
pub struct Tooltip<'a> {
    text: &'a str,
    anchor: Rect,
}

impl<'a> Tooltip<'a> {
    pub fn new(text: &'a str, anchor: Rect) -> Tooltip<'a> {
        Tooltip { text, anchor }
    }
}

impl Widget for Tooltip<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            let w = (text::width(self.text) as u16 + 2).min(area.width);
            let chip = place(self.anchor, (w, 1), area);
            Clear.render(chip, buf);
            let style = Style::new()
                .fg(t.text(TextRole::Secondary))
                .bg(t.surface(Surface::Popover));
            buf.set_style(chip, style);
            buf.set_string(
                chip.x + 1,
                chip.y,
                text::truncate(self.text, chip.width.saturating_sub(2) as usize),
                style,
            );
        });
    }
}
