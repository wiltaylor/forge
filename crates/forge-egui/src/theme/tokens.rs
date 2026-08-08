//! GENERATED FILE — do not edit by hand.
//! Source:     packages/tokens/tokens.source.mjs
//! Regenerate: just generate   (`just check` fails while this file is stale)
//!
//! Geometry, type, motion and control-height tokens.
//!
//! These are the tokens a pixel canvas can express. A terminal cannot, so
//! forge-tui has no counterpart of this file. Lengths are egui points, one
//! per pixel of the token source; durations are seconds.
//!
//! Each field names the token it carries. Where a token name cannot be a
//! Rust identifier the field reverses it: `--fs-2xl` is `xl2`.

/// Corner radii. A pill is `height / 2.0` at the call site, not a token.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radius {
    /// `--r-sm`
    pub sm: f32,
    /// `--r-md`
    pub md: f32,
    /// `--r-lg`
    pub lg: f32,
}

impl Default for Radius {
    fn default() -> Self {
        Radius {
            sm: 4.0,
            md: 6.0,
            lg: 8.0,
        }
    }
}

/// The spacing scale, held as its base step. `space.x(n)` is n steps,
/// which is how the rest of the `--sp-*` ramp is reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    /// `--sp-1`
    pub base: f32,
}

impl Default for Space {
    fn default() -> Self {
        Space { base: 4.0 }
    }
}

/// The type scale. `xs`..`lg` are the body sizes; `xl`, `xl2` and `xl3`
/// are the heading sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeScale {
    /// `--fs-xs`
    pub xs: f32,
    /// `--fs-sm`
    pub sm: f32,
    /// `--fs-base`
    pub base: f32,
    /// `--fs-md`
    pub md: f32,
    /// `--fs-lg`
    pub lg: f32,
    /// `--fs-xl`
    pub xl: f32,
    /// `--fs-2xl`
    pub xl2: f32,
    /// `--fs-3xl`
    pub xl3: f32,
}

impl Default for TypeScale {
    fn default() -> Self {
        TypeScale {
            xs: 11.0,
            sm: 12.0,
            base: 14.0,
            md: 16.0,
            lg: 18.0,
            xl: 22.0,
            xl2: 28.0,
            xl3: 34.0,
        }
    }
}

/// Control heights — the height a button, input or select stands at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlHeights {
    /// `--h-sm`
    pub sm: f32,
    /// `--h-md`
    pub md: f32,
    /// `--h-lg`
    pub lg: f32,
    /// `--h-xl`
    pub xl: f32,
}

impl Default for ControlHeights {
    fn default() -> Self {
        ControlHeights {
            sm: 28.0,
            md: 32.0,
            lg: 36.0,
            xl: 40.0,
        }
    }
}

/// Motion durations, in seconds. The source authors milliseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionDurations {
    /// `--dur-1`
    pub fast: f32,
    /// `--dur-2`
    pub base: f32,
    /// `--dur-3`
    pub slow: f32,
}

impl Default for MotionDurations {
    fn default() -> Self {
        MotionDurations {
            fast: 0.08,
            base: 0.16,
            slow: 0.24,
        }
    }
}

// Shell dimensions. The rail and the status bar are scoped to this kit in
// the token source: the web shell has no equivalent of either.

/// `--sidebar-w`
pub const SIDEBAR_WIDTH: f32 = 240.0;

/// `--sidebar-rail-w` — collapsed sidebar.
pub const SIDEBAR_RAIL: f32 = 56.0;

/// `--topbar-h`
pub const TOPBAR_HEIGHT: f32 = 48.0;

/// `--statusbar-h`
pub const STATUSBAR_HEIGHT: f32 = 28.0;
