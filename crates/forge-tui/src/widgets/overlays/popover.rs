use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, Widget};

/// Place a floating rect of `size` next to `anchor` within `bounds`:
/// below-left-aligned preferred, flipping up / shifting left when out of
/// room. Shared by Popover, Tooltip, and the menus.
pub fn place(anchor: Rect, size: (u16, u16), bounds: Rect) -> Rect {
    let (w, h) = (size.0.min(bounds.width), size.1.min(bounds.height));
    let below = anchor.y + anchor.height;
    let y = if below + h <= bounds.y + bounds.height {
        below
    } else if anchor.y >= bounds.y + h {
        anchor.y - h
    } else {
        (bounds.y + bounds.height).saturating_sub(h).max(bounds.y)
    };
    let max_x = (bounds.x + bounds.width).saturating_sub(w);
    let x = anchor.x.min(max_x).max(bounds.x);
    Rect::new(x, y, w, h)
}

/// Anchored floating panel. Chrome only; render content into
/// [`Popover::inner`]. Not modal by default — pair with the runtime overlay
/// stack for click-away/Esc semantics.
#[derive(Clone, Debug)]
pub struct Popover<'a> {
    anchor: Rect,
    size: (u16, u16),
    title: Option<&'a str>,
    theme: Option<&'a Theme>,
}

impl<'a> Popover<'a> {
    pub fn new(anchor: Rect) -> Popover<'a> {
        Popover {
            anchor,
            size: (30, 8),
            title: None,
            theme: None,
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.size = (width, height);
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

    pub fn panel(&self, bounds: Rect) -> Rect {
        place(self.anchor, self.size, bounds)
    }

    fn block(&self, t: &'a Theme) -> Block<'a> {
        let mut block = Block::bordered()
            .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
            .style(Style::new().bg(t.bg[4]));
        if let Some(title) = self.title {
            block = block
                .title(title)
                .title_style(Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD));
        }
        block
    }

    pub fn inner(&self, bounds: Rect) -> Rect {
        let t = self.theme.unwrap_or_else(|| default_theme());
        self.block(t).inner(self.panel(bounds))
    }
}

impl Widget for Popover<'_> {
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
