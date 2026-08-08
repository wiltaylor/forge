use crate::theme::TextRole;
use crate::widgets::paint;
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
pub struct Icon {
    glyph: Glyph,
    color: Option<Color>,
}

impl Icon {
    pub fn new(glyph: Glyph) -> Icon {
        Icon { glyph, color: None }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for Icon {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            let color = self.color.unwrap_or(t.text(TextRole::Secondary));
            buf.set_string(area.x, area.y, self.glyph.as_str(), Style::new().fg(color));
        });
    }
}
