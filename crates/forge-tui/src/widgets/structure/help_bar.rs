use crate::event::Keymap;
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// One-row keybind cheatsheet generated from a [`Keymap`]:
/// `⌃K palette · / search · ? help`. Entries that don't fit are dropped.
#[derive(Clone, Debug)]
pub struct HelpBar<'a> {
    keymap: &'a Keymap,
    theme: Option<&'a Theme>,
}

impl<'a> HelpBar<'a> {
    pub fn new(keymap: &'a Keymap) -> HelpBar<'a> {
        HelpBar { keymap, theme: None }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for HelpBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let right = area.x + area.width;
        let mut x = area.x;
        for (i, binding) in self.keymap.bindings().iter().enumerate() {
            let combo = binding.combo.to_string();
            let need = text::width(&combo) as u16 + 1 + text::width(binding.help) as u16;
            let sep = if i > 0 { 3 } else { 0 };
            if x + sep + need > right {
                break;
            }
            if i > 0 {
                buf.set_string(x + 1, area.y, "·", Style::new().fg(t.fg[3]));
                x += sep;
            }
            buf.set_string(x, area.y, &combo, Style::new().fg(t.fg[1]).bg(t.bg[3]));
            x += text::width(&combo) as u16 + 1;
            buf.set_string(x, area.y, binding.help, Style::new().fg(t.fg[2]));
            x += text::width(binding.help) as u16;
        }
    }
}
