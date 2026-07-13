use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Curated single-cell glyph set. Everything here is single-width in
/// unicode-width terms; emoji-presentation forcing (VS16) is deliberately
/// avoided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Check,
    Cross,
    Warn,
    Info,
    Dot,
    Circle,
    ChevronRight,
    ChevronLeft,
    ChevronUp,
    ChevronDown,
    ArrowUp,
    ArrowDown,
    Plus,
    Minus,
    Ellipsis,
}

impl Glyph {
    pub const fn as_str(self) -> &'static str {
        match self {
            Glyph::Check => "✓",
            Glyph::Cross => "✗",
            Glyph::Warn => "⚠",
            Glyph::Info => "ℹ",
            Glyph::Dot => "•",
            Glyph::Circle => "○",
            Glyph::ChevronRight => "▸",
            Glyph::ChevronLeft => "◂",
            Glyph::ChevronUp => "▴",
            Glyph::ChevronDown => "▾",
            Glyph::ArrowUp => "↑",
            Glyph::ArrowDown => "↓",
            Glyph::Plus => "+",
            Glyph::Minus => "−",
            Glyph::Ellipsis => "…",
        }
    }
}

/// A single themed glyph.
#[derive(Clone, Debug)]
pub struct Icon<'a> {
    glyph: Glyph,
    color: Option<Color>,
    theme: Option<&'a Theme>,
}

impl<'a> Icon<'a> {
    pub fn new(glyph: Glyph) -> Icon<'a> {
        Icon {
            glyph,
            color: None,
            theme: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Icon<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let color = self.color.unwrap_or(t.fg[1]);
        buf.set_string(area.x, area.y, self.glyph.as_str(), Style::new().fg(color));
    }
}
