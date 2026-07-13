use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, Padding, Widget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
    Bottom,
}

/// Docked panel (detail drawers, filter panes). Chrome only — render content
/// into [`Sheet::inner`].
#[derive(Clone, Debug)]
pub struct Sheet<'a> {
    side: Side,
    size: u16,
    title: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Sheet<'a> {
    pub fn new(side: Side) -> Sheet<'a> {
        Sheet {
            side,
            size: 36,
            title: None,
            theme: None,
        }
    }

    /// Width (left/right) or height (bottom) of the docked panel.
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn panel(&self, area: Rect) -> Rect {
        match self.side {
            Side::Right => {
                let w = self.size.min(area.width);
                Rect::new(area.x + area.width - w, area.y, w, area.height)
            }
            Side::Left => Rect::new(area.x, area.y, self.size.min(area.width), area.height),
            Side::Bottom => {
                let h = self.size.min(area.height);
                Rect::new(area.x, area.y + area.height - h, area.width, h)
            }
        }
    }

    fn block(&self, t: &'a Theme) -> Block<'a> {
        let mut block = Block::bordered()
            .border_style(Style::new().fg(t.border.strong).bg(t.bg[1]))
            .style(Style::new().bg(t.bg[1]))
            .padding(Padding::horizontal(1));
        if let Some(title) = self.title {
            block = block
                .title(title)
                .title_style(Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD));
        }
        block
    }

    pub fn inner(&self, area: Rect) -> Rect {
        let t = self.theme.unwrap_or_else(|| default_theme());
        self.block(t).inner(self.panel(area))
    }
}

impl Widget for Sheet<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let panel = self.panel(area);
        Clear.render(panel, buf);
        self.block(t).render(panel, buf);
    }
}
