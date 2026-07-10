use crate::text;
use crate::theme::{default_theme, Theme};
use crate::widgets::primitives::Glyph;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Centered empty-state: icon, title, and a dim hint.
#[derive(Clone, Debug)]
pub struct Empty<'a> {
    title: &'a str,
    hint: Option<&'a str>,
    glyph: Glyph,
    theme: Option<&'a Theme>,
}

impl<'a> Empty<'a> {
    pub fn new(title: &'a str) -> Empty<'a> {
        Empty { title, hint: None, glyph: Glyph::Circle, theme: None }
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn glyph(mut self, glyph: Glyph) -> Self {
        self.glyph = glyph;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Empty<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let rows: u16 = 2 + u16::from(self.hint.is_some());
        let top = area.y + area.height.saturating_sub(rows) / 2;
        let center = |s: &str, y: u16, style: Style, buf: &mut Buffer| {
            if y >= area.y + area.height {
                return;
            }
            let s = text::truncate(s, area.width as usize);
            let x = area.x + area.width.saturating_sub(text::width(&s) as u16) / 2;
            buf.set_string(x, y, s, style);
        };
        center(self.glyph.as_str(), top, Style::new().fg(t.fg[3]), buf);
        center(self.title, top + 1, Style::new().fg(t.fg[1]), buf);
        if let Some(hint) = self.hint {
            center(hint, top + 2, Style::new().fg(t.fg[2]), buf);
        }
    }
}
