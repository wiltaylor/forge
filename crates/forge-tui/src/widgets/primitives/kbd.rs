use crate::event::KeyCombo;
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use std::borrow::Cow;

/// Key-cap chip: ` ⌃K ` on a raised background.
#[derive(Clone, Debug)]
pub struct Kbd<'a> {
    keys: Cow<'a, str>,
    theme: Option<&'a Theme>,
}

impl<'a> Kbd<'a> {
    pub fn new(keys: &'a str) -> Kbd<'a> {
        Kbd {
            keys: Cow::Borrowed(keys),
            theme: None,
        }
    }

    pub fn combo(combo: KeyCombo) -> Kbd<'a> {
        Kbd {
            keys: Cow::Owned(combo.to_string()),
            theme: None,
        }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Natural width in cells, for layout math.
    pub fn width(&self) -> u16 {
        (text::width(&self.keys) as u16).saturating_add(2)
    }
}

impl Widget for Kbd<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let style = Style::new().fg(t.fg[1]).bg(t.bg[3]);
        let label = text::truncate(&self.keys, area.width.saturating_sub(2) as usize);
        let w = (text::width(&label) as u16 + 2).min(area.width);
        buf.set_style(Rect::new(area.x, area.y, w, 1), style);
        buf.set_string(area.x + 1, area.y, label, style);
    }
}
