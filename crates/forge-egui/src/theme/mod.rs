//! The Forge theme: a Rust mirror of `packages/tokens/src/theme.ts`, sibling
//! of `forge-tui/src/theme` — same token layout, real alpha instead of
//! terminal pre-blending, plus the geometry/typography tokens a pixel canvas
//! can express (radii, spacing, type scale, control heights, motion).
//!
//! Install a theme once with [`Theme::apply`]; widgets read it back with
//! [`Theme::of`]. Overrides use plain struct-update syntax — Rust's native
//! "DeepPartial":
//!
//! ```
//! use forge_egui::theme::{Accent, Theme};
//! use egui::Color32;
//! let custom = Theme {
//!     accent: Accent { base: Color32::from_rgb(0x8A, 0x63, 0xD2), ..Theme::dark().accent },
//!     ..Theme::dark()
//! };
//! ```

mod apply;
mod chart_palette;
pub mod color;
mod fonts;
mod palette;
mod tokens;

pub use apply::scrim;
pub use chart_palette::{chart_series, series_color, CHART_SERIES_LEN};
pub use color::{blend, rgb, shift};
pub use tokens::{
    ControlHeights, FontWeight, MotionDurations, Radius, Space, TypeScale, SIDEBAR_RAIL,
    SIDEBAR_WIDTH, STATUSBAR_HEIGHT, TOPBAR_HEIGHT,
};

use egui::Color32;

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
    pub base: Color32,
    /// Translucent surface tint (real alpha; composites over any surface).
    pub bg: Color32,
    /// Text readable on the tint.
    pub fg: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accent {
    pub base: Color32,
    pub hover: Color32,
    pub press: Color32,
    /// Selection/tint background (translucent, real alpha).
    pub bg: Color32,
    /// Accent-tinted text.
    pub fg: Color32,
    /// Text on solid accent fills.
    pub contrast: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderTokens {
    pub subtle: Color32,
    pub default: Color32,
    pub strong: Color32,
}

/// The full Forge token set. Field layout mirrors the web `Theme` interface
/// and forge-tui: `bg` rises page(0) → popover(4), `fg` descends primary(0)
/// → disabled(3).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub scheme: Scheme,
    pub bg: [Color32; 5],
    pub fg: [Color32; 4],
    pub border: BorderTokens,
    pub accent: Accent,
    pub success: SemanticTriple,
    pub warning: SemanticTriple,
    pub danger: SemanticTriple,
    pub info: SemanticTriple,
    pub radius: Radius,
    pub space: Space,
    pub type_scale: TypeScale,
    pub control: ControlHeights,
    pub motion: MotionDurations,
}

impl Theme {
    pub fn dark() -> Theme {
        palette::dark()
    }

    pub fn light() -> Theme {
        palette::light()
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
    pub fn with_accent(self, base: Color32) -> Theme {
        let toward_fg = match self.scheme {
            Scheme::Dark => 1.0,
            Scheme::Light => -1.0,
        };
        Theme {
            accent: Accent {
                base,
                hover: shift(base, 0.10 * toward_fg),
                press: shift(base, -0.12 * toward_fg),
                bg: color::with_alpha(base, 36), // ≈ 14%
                fg: shift(base, 0.45 * toward_fg),
                contrast: self.accent.contrast,
            },
            ..self
        }
    }

    /// A [`egui::FontId`] for the given size using the Forge sans family at
    /// the requested weight. Falls back to the proportional default when the
    /// `fonts` feature is off — or when the Forge fonts aren't (yet) bound on
    /// this context, so text never panics on an unbound named family.
    pub fn font(&self, ctx: &egui::Context, weight: FontWeight, size: f32) -> egui::FontId {
        let family = fonts::family(weight);
        let family = match family {
            egui::FontFamily::Name(_) if !fonts::bound(ctx, &family) => {
                egui::FontFamily::Proportional
            }
            f => f,
        };
        egui::FontId::new(size, family)
    }

    /// The monospace [`egui::FontId`].
    pub fn mono(&self, size: f32) -> egui::FontId {
        egui::FontId::new(size, egui::FontFamily::Monospace)
    }
}

impl Default for Theme {
    fn default() -> Theme {
        Theme::dark()
    }
}
