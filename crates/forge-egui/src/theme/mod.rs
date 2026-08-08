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

/// Named background roles — the meaning of each step of the `bg` ramp.
///
/// Paint with [`Theme::surface`] instead of indexing the ramp, so a widget
/// says which surface it is on rather than which array slot that surface
/// happens to occupy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// The page behind everything.
    Page,
    /// A card, panel or bar sitting on the page.
    Card,
    /// A hovered row, or a card nested inside a card.
    Hover,
    /// A pressed control, or the active row of a list.
    Pressed,
    /// A popover, dropdown or menu floating above the page.
    Popover,
}

/// Named foreground roles — the meaning of each step of the `fg` ramp.
///
/// Read with [`Theme::text`]. Named `TextRole` rather than `Text` to match
/// forge-tui, where the short name would collide with `ratatui::text::Text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    /// Primary text — body copy, values, headings.
    Primary,
    /// Secondary text — labels and supporting copy.
    Secondary,
    /// Tertiary text — captions, hints, timestamps.
    Tertiary,
    /// Disabled text and placeholders.
    Disabled,
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
/// → disabled(3). Read those two ramps through [`Theme::surface`] and
/// [`Theme::text`], which name each step.
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

    /// The background color for a named [`Surface`].
    pub fn surface(&self, s: Surface) -> Color32 {
        match s {
            Surface::Page => self.bg[0],
            Surface::Card => self.bg[1],
            Surface::Hover => self.bg[2],
            Surface::Pressed => self.bg[3],
            Surface::Popover => self.bg[4],
        }
    }

    /// The foreground color for a named [`TextRole`].
    pub fn text(&self, r: TextRole) -> Color32 {
        match r {
            TextRole::Primary => self.fg[0],
            TextRole::Secondary => self.fg[1],
            TextRole::Tertiary => self.fg[2],
            TextRole::Disabled => self.fg[3],
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
