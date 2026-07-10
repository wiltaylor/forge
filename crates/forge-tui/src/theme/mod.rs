//! The Forge theme: a Rust mirror of `packages/tokens/src/theme.ts`.
//!
//! Widgets take a theme via their `.theme(&theme)` builder method; without
//! one they fall back to the process-wide default (set once by
//! [`set_default_theme`], usually from `runtime::run`). Overrides use plain
//! struct-update syntax — Rust's native "DeepPartial":
//!
//! ```
//! use forge_tui::theme::{Theme, Accent};
//! use ratatui::style::Color;
//! let custom = Theme {
//!     accent: Accent { base: Color::Rgb(0x8A, 0x63, 0xD2), ..Theme::dark().accent },
//!     ..Theme::dark()
//! };
//! ```

mod chart_palette;
pub mod color;
mod palette;

pub use chart_palette::{chart_series, series_color, CHART_SERIES_LEN};
pub use color::{blend, quantize, rgb, shift, ColorMode};

use ratatui::style::Color;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Dark,
    Light,
}

/// Semantic tone selector used by Badge, Alert, Toast, StatusDot, …
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Success,
    Warning,
    Danger,
    Info,
}

/// A semantic color triple: solid tone, surface tint, and text-on-tint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticTriple {
    /// Solid tone — borders, icons, gauge fills.
    pub base: Color,
    /// Tint background (alpha pre-blended over `bg[1]`).
    pub bg: Color,
    /// Text readable on the tint.
    pub fg: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accent {
    pub base: Color,
    pub hover: Color,
    pub press: Color,
    /// Selection/tint background (alpha pre-blended over `bg[1]`).
    pub bg: Color,
    /// Accent-tinted text.
    pub fg: Color,
    /// Text on solid accent fills.
    pub contrast: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderTokens {
    pub subtle: Color,
    pub default: Color,
    pub strong: Color,
}

/// The full Forge token set. Field layout mirrors the web `Theme` interface:
/// `bg` rises page(0) → popover(4), `fg` descends primary(0) → disabled(3).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub scheme: Scheme,
    pub bg: [Color; 5],
    pub fg: [Color; 4],
    pub border: BorderTokens,
    pub accent: Accent,
    pub success: SemanticTriple,
    pub warning: SemanticTriple,
    pub danger: SemanticTriple,
    pub info: SemanticTriple,
}

impl Theme {
    pub const fn dark() -> Theme {
        palette::DARK
    }

    pub const fn light() -> Theme {
        palette::LIGHT
    }

    pub fn severity(&self, s: Severity) -> &SemanticTriple {
        match s {
            Severity::Success => &self.success,
            Severity::Warning => &self.warning,
            Severity::Danger => &self.danger,
            Severity::Info => &self.info,
        }
    }

    /// Derive a theme with a custom accent; hover/press/fg/bg are derived by
    /// lightness shifts and re-tinting so a single brand color is enough.
    pub fn with_accent(self, base: Color) -> Theme {
        let toward_fg = match self.scheme {
            Scheme::Dark => 1.0,
            Scheme::Light => -1.0,
        };
        Theme {
            accent: Accent {
                base,
                hover: shift(base, 0.10 * toward_fg),
                press: shift(base, -0.12 * toward_fg),
                bg: blend(base, self.bg[1], 0.14),
                fg: shift(base, 0.45 * toward_fg),
                contrast: self.accent.contrast,
            },
            ..self
        }
    }

    /// Degrade the theme for the terminal's color capability. Truecolor is a
    /// no-op; 256-color quantizes every token (grayscale-ramp-aware, so the
    /// five near-black backgrounds stay distinct); 16-color swaps in a
    /// semantic ANSI mapping that mirrors `packages/term/src/theme.ts`.
    pub fn quantized(&self, mode: ColorMode) -> Theme {
        match mode {
            ColorMode::TrueColor => self.clone(),
            ColorMode::Indexed256 => {
                let mut q = self.map(|c| quantize(c, mode));
                q.bg = self.quantized_bg_ramp();
                q
            }
            ColorMode::Ansi16 => self.ansi16(),
        }
    }

    /// Quantize the bg ramp with collision avoidance: several of the
    /// near-black Forge backgrounds share the same nearest 256-color gray, so
    /// each *distinct* source color claims the nearest index not already
    /// taken (identical sources — e.g. light bg1/bg4, both white — share).
    fn quantized_bg_ramp(&self) -> [Color; 5] {
        let mut used: Vec<u8> = Vec::new();
        let mut assigned: Vec<(Color, u8)> = Vec::new();
        self.bg.map(|c| {
            let Color::Rgb(r, g, b) = c else { return c };
            if let Some((_, idx)) = assigned.iter().find(|(src, _)| *src == c) {
                return Color::Indexed(*idx);
            }
            let idx = color::nearest_indexed_excluding(r, g, b, &used);
            used.push(idx);
            assigned.push((c, idx));
            Color::Indexed(idx)
        })
    }

    fn map(&self, f: impl Fn(Color) -> Color) -> Theme {
        let tri = |t: &SemanticTriple| SemanticTriple {
            base: f(t.base),
            bg: f(t.bg),
            fg: f(t.fg),
        };
        Theme {
            name: self.name,
            scheme: self.scheme,
            bg: self.bg.map(&f),
            fg: self.fg.map(&f),
            border: BorderTokens {
                subtle: f(self.border.subtle),
                default: f(self.border.default),
                strong: f(self.border.strong),
            },
            accent: Accent {
                base: f(self.accent.base),
                hover: f(self.accent.hover),
                press: f(self.accent.press),
                bg: f(self.accent.bg),
                fg: f(self.accent.fg),
                contrast: f(self.accent.contrast),
            },
            success: tri(&self.success),
            warning: tri(&self.warning),
            danger: tri(&self.danger),
            info: tri(&self.info),
        }
    }

    /// Semantic 16-color mapping: danger→red, success→green, warning→yellow,
    /// accent→blue, info→cyan; `-fg` variants → bright counterparts; the
    /// background ramp collapses onto the terminal default + black/dark-gray.
    fn ansi16(&self) -> Theme {
        use Color::*;
        let (bg, raised, dim, text, muted) = match self.scheme {
            Scheme::Dark => (Reset, Black, DarkGray, White, Gray),
            // On light terminals ANSI "White"/"Black" invert roles.
            Scheme::Light => (Reset, White, Gray, Black, DarkGray),
        };
        Theme {
            name: self.name,
            scheme: self.scheme,
            bg: [bg, bg, raised, raised, raised],
            fg: [text, muted, dim, dim],
            border: BorderTokens {
                subtle: dim,
                default: dim,
                strong: muted,
            },
            accent: Accent {
                base: Blue,
                hover: LightBlue,
                press: Blue,
                bg: raised,
                fg: LightBlue,
                contrast: text,
            },
            success: SemanticTriple { base: Green, bg: raised, fg: LightGreen },
            warning: SemanticTriple { base: Yellow, bg: raised, fg: LightYellow },
            danger: SemanticTriple { base: Red, bg: raised, fg: LightRed },
            info: SemanticTriple { base: Cyan, bg: raised, fg: LightCyan },
        }
    }
}

impl Default for Theme {
    fn default() -> Theme {
        Theme::dark()
    }
}

static DEFAULT_THEME: OnceLock<Theme> = OnceLock::new();

/// The process-wide fallback theme used by widgets built without an explicit
/// `.theme(...)`. Dark until [`set_default_theme`] is called.
pub fn default_theme() -> &'static Theme {
    DEFAULT_THEME.get_or_init(Theme::dark)
}

/// Set the process-wide default theme (once — typically from `runtime::run`
/// with the quantized theme). Returns the rejected theme if one was already
/// set or defaulted.
pub fn set_default_theme(theme: Theme) -> Result<(), Theme> {
    DEFAULT_THEME.set(theme)
}
