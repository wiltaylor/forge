use crate::theme::{chart_series, default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use unicode_segmentation::UnicodeSegmentation;

/// Initials chip with a deterministic per-name hue: ` WT ` on a colored fill.
#[derive(Clone, Debug)]
pub struct Avatar<'a> {
    name: &'a str,
    theme: Option<&'a Theme>,
}

impl<'a> Avatar<'a> {
    pub fn new(name: &'a str) -> Avatar<'a> {
        Avatar { name, theme: None }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn initials(&self) -> String {
        let mut words = self.name.split_whitespace();
        let first = words.next().unwrap_or("?");
        let second = words.last();
        let lead = |s: &str| {
            s.graphemes(true)
                .next()
                .map(|g| g.to_uppercase())
                .unwrap_or_default()
        };
        match second {
            Some(s) => format!("{}{}", lead(first), lead(s)),
            None => lead(first),
        }
    }
}

impl Widget for Avatar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let hues = chart_series(t);
        let hash: usize = self
            .name
            .bytes()
            .fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
        let bg = hues[hash % hues.len()];
        let style = Style::new()
            .fg(t.accent.contrast)
            .bg(bg)
            .add_modifier(Modifier::BOLD);
        let initials = self.initials();
        let w = (initials.chars().count() as u16 + 2).min(area.width);
        buf.set_style(Rect::new(area.x, area.y, w, 1), style);
        buf.set_string(area.x + 1, area.y, initials, style);
    }
}
