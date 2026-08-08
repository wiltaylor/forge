use crate::text;
use crate::theme::TextRole;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Dense definition list: dim keys, bright values, keys column auto-sized.
#[derive(Clone, Debug)]
pub struct KeyValue<'a> {
    pairs: &'a [(&'a str, &'a str)],
}

impl<'a> KeyValue<'a> {
    pub fn new(pairs: &'a [(&'a str, &'a str)]) -> KeyValue<'a> {
        KeyValue { pairs }
    }
}

impl Widget for KeyValue<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            let key_w = self
                .pairs
                .iter()
                .map(|(k, _)| text::width(k))
                .max()
                .unwrap_or(0)
                .min(area.width as usize / 2) as u16;
            for (i, (k, v)) in self.pairs.iter().enumerate() {
                let y = area.y + i as u16;
                if y >= area.y + area.height {
                    break;
                }
                buf.set_string(
                    area.x,
                    y,
                    text::truncate(k, key_w as usize),
                    Style::new().fg(t.text(TextRole::Tertiary)),
                );
                if area.width > key_w + 2 {
                    buf.set_string(
                        area.x + key_w + 2,
                        y,
                        text::truncate(v, (area.width - key_w - 2) as usize),
                        Style::new().fg(t.text(TextRole::Primary)),
                    );
                }
            }
        });
    }
}
