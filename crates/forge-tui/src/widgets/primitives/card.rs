use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Padding, Widget};

/// A flat, border-driven surface (`bg[1]`), the Forge card. Renders chrome
/// only — measure the content region with [`Card::inner`] and render content
/// there yourself.
#[derive(Clone, Debug, Default)]
pub struct Card<'a> {
    title: Option<&'a str>,
    footer: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Card<'a> {
    pub fn new() -> Card<'a> {
        Card::default()
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn footer(mut self, footer: &'a str) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn block(&self, t: &Theme) -> Block<'a> {
        let mut block = Block::bordered()
            .border_style(Style::new().fg(t.border.default))
            .style(Style::new().bg(t.bg[1]))
            .padding(Padding::horizontal(1));
        if let Some(title) = self.title {
            block = block.title(title).title_style(
                Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD),
            );
        }
        if let Some(footer) = self.footer {
            block = block
                .title_bottom(footer)
                .title_style(Style::new().fg(t.fg[2]));
        }
        block
    }

    /// The content region inside the border and padding.
    pub fn inner(&self, area: Rect) -> Rect {
        let t = self.theme.unwrap_or_else(|| default_theme());
        self.block(t).inner(area)
    }
}

impl Widget for Card<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        self.block(t).render(area, buf);
    }
}
