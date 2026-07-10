use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, Padding, Widget};

/// Centered dialog chrome on the popover surface. Renders the panel only —
/// measure the body region with [`Modal::inner`] and render content there.
/// Pair with `runtime::dim` (or open it through the runtime overlay stack,
/// which dims for you).
#[derive(Clone, Debug)]
pub struct Modal<'a> {
    title: Option<&'a str>,
    footer: Option<&'a str>,
    width: u16,
    height: u16,
    theme: Option<&'a Theme>,
}

impl<'a> Modal<'a> {
    pub fn new() -> Modal<'a> {
        Modal { title: None, footer: None, width: 56, height: 10, theme: None }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    /// Hint row inside the bottom border (e.g. `Esc close · Enter confirm`).
    pub fn footer(mut self, footer: &'a str) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Desired panel size; clamped to the available area.
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// The centered panel rect within `area`.
    pub fn panel(&self, area: Rect) -> Rect {
        let w = self.width.min(area.width.saturating_sub(2)).max(1);
        let h = self.height.min(area.height.saturating_sub(1)).max(1);
        Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        )
    }

    fn block(&self, t: &'a Theme) -> Block<'a> {
        let mut block = Block::bordered()
            .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
            .style(Style::new().bg(t.bg[4]))
            .padding(Padding::horizontal(1));
        if let Some(title) = self.title {
            block = block
                .title(title)
                .title_style(Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD));
        }
        if let Some(footer) = self.footer {
            block = block
                .title_bottom(footer)
                .title_style(Style::new().fg(t.fg[2]));
        }
        block
    }

    /// The content region inside the panel.
    pub fn inner(&self, area: Rect) -> Rect {
        let t = self.theme.unwrap_or_else(|| default_theme());
        self.block(t).inner(self.panel(area))
    }
}

impl Default for Modal<'_> {
    fn default() -> Self {
        Modal::new()
    }
}

impl Widget for Modal<'_> {
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
