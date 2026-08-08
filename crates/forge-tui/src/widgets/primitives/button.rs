use crate::text;
use crate::theme::{Surface, TextRole, Theme};
use crate::widgets::paint;
use crate::widgets::primitives::Glyph;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Widget};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    #[default]
    Default,
    Primary,
    Ghost,
    Danger,
}

/// A push button. One row renders as a filled chip; three or more rows render
/// bordered. Activation (Enter/Space) is the caller's job — the button is a
/// pure view, focus/press state comes in through the builder.
#[derive(Clone, Debug)]
pub struct Button<'a> {
    label: &'a str,
    variant: Variant,
    focused: bool,
    disabled: bool,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Button<'a> {
        Button {
            label,
            variant: Variant::Default,
            focused: false,
            disabled: false,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Natural width in cells (label + one cell padding each side), for
    /// layout math.
    pub fn width(&self) -> u16 {
        (text::width(self.label) as u16).saturating_add(2)
    }

    fn style(&self, t: &Theme) -> Style {
        if self.disabled {
            return Style::new()
                .fg(t.text(TextRole::Disabled))
                .bg(t.surface(Surface::Hover));
        }
        let s = match self.variant {
            Variant::Primary => Style::new().fg(t.accent.contrast).bg(if self.focused {
                t.accent.hover
            } else {
                t.accent.base
            }),
            Variant::Danger => Style::new().fg(t.accent.contrast).bg(t.danger.base),
            Variant::Default => Style::new()
                .fg(t.text(TextRole::Primary))
                .bg(if self.focused {
                    t.surface(Surface::Popover)
                } else {
                    t.surface(Surface::Pressed)
                }),
            Variant::Ghost => Style::new().fg(if self.focused {
                t.accent.fg
            } else {
                t.text(TextRole::Secondary)
            }),
        };
        if self.focused {
            s.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            s
        }
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            let style = self.style(t);
            if area.height >= 3 {
                let border = if self.disabled {
                    t.border.subtle
                } else if self.focused {
                    t.accent.base
                } else {
                    t.border.strong
                };
                let block = Block::bordered().border_style(Style::new().fg(border));
                let inner = block.inner(area);
                block.render(area, buf);
                let label = text::truncate(self.label, inner.width as usize);
                let row = inner.y + inner.height / 2;
                let x = inner.x + (inner.width.saturating_sub(text::width(&label) as u16)) / 2;
                buf.set_style(Rect::new(inner.x, row, inner.width, 1), style);
                buf.set_string(x, row, label, style);
            } else {
                let label = text::truncate(self.label, area.width.saturating_sub(2) as usize);
                buf.set_style(Rect::new(area.x, area.y, area.width, 1), style);
                let x = area.x + (area.width.saturating_sub(text::width(&label) as u16)) / 2;
                buf.set_string(x, area.y, label, style);
            }
        });
    }
}

/// A square single-glyph button.
#[derive(Clone, Debug)]
pub struct IconButton {
    glyph: Glyph,
    variant: Variant,
    focused: bool,
    disabled: bool,
}

impl IconButton {
    pub fn new(glyph: Glyph) -> IconButton {
        IconButton {
            glyph,
            variant: Variant::Default,
            focused: false,
            disabled: false,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Widget for IconButton {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let glyph = self.glyph.as_str();
        let button = Button {
            label: glyph,
            variant: self.variant,
            focused: self.focused,
            disabled: self.disabled,
        };
        // Constrain to a 3-cell chip so it stays square-ish.
        let w = area.width.min(3);
        button.render(Rect::new(area.x, area.y, w, area.height), buf);
    }
}
