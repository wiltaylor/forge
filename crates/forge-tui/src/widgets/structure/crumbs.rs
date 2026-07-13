use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Breadcrumb path: `infra › nodes › node-3`, the last segment bright.
#[derive(Clone, Debug)]
pub struct Crumbs<'a> {
    segments: &'a [&'a str],
    theme: Option<&'a Theme>,
}

impl<'a> Crumbs<'a> {
    pub fn new(segments: &'a [&'a str]) -> Crumbs<'a> {
        Crumbs {
            segments,
            theme: None,
        }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Crumbs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let right = area.x + area.width;
        let mut x = area.x;
        for (i, seg) in self.segments.iter().enumerate() {
            if x >= right {
                break;
            }
            let last = i + 1 == self.segments.len();
            let style = Style::new().fg(if last { t.fg[0] } else { t.fg[2] });
            let seg = text::truncate(seg, (right - x) as usize);
            buf.set_string(x, area.y, &seg, style);
            x += text::width(&seg) as u16;
            if !last && x + 3 <= right {
                buf.set_string(x, area.y, " › ", Style::new().fg(t.fg[3]));
                x += 3;
            }
        }
    }
}
