//! Curated glyph set and the single-glyph [`Icon`] widget. Forge carries no
//! icon-font dependency — these are plain text glyphs covered by the bundled
//! fonts plus egui's built-in symbol fallback.

use crate::theme::Theme;
use egui::{Color32, Ui};

/// Curated glyph set shared across the kit (chevrons, status marks, …).
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
    Search,
    Gear,
    Folder,
    File,
    Terminal,
    Bolt,
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
            Glyph::Search => "◎",
            Glyph::Gear => "⚙",
            Glyph::Folder => "▤",
            Glyph::File => "▢",
            Glyph::Terminal => "❯",
            Glyph::Bolt => "⚡",
        }
    }
}

/// A single themed glyph, sized like body text by default.
pub struct Icon {
    glyph: Glyph,
    color: Option<Color32>,
    size: Option<f32>,
}

impl Icon {
    pub fn new(glyph: Glyph) -> Icon {
        Icon {
            glyph,
            color: None,
            size: None,
        }
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let color = self.color.unwrap_or(t.fg[1]);
        let size = self.size.unwrap_or(t.type_scale.base);
        ui.label(
            egui::RichText::new(self.glyph.as_str())
                .size(size)
                .color(color),
        )
    }
}
