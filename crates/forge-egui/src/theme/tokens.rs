//! Geometry, typography, and motion tokens — the parts of
//! `packages/tokens/css/tokens.css` a terminal couldn't express. All values
//! are in egui points.

/// Corner radii (`--r-sm/md/lg`). Pill = `height / 2.0` at the call site.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radius {
    pub sm: f32,
    pub md: f32,
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

/// 4-point spacing scale (`--sp-*`). `space.x(n)` = n × 4pt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    pub base: f32,
}

impl Space {
    pub fn x(&self, n: f32) -> f32 {
        self.base * n
    }
}

impl Default for Space {
    fn default() -> Self {
        Space { base: 4.0 }
    }
}

/// Type scale (`--fs-*`): 1.2 ratio anchored at 14. `xs..xl` are body sizes,
/// `h3..h1` the heading sizes (22/28/34).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeScale {
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub md: f32,
    pub lg: f32,
    pub h3: f32,
    pub h2: f32,
    pub h1: f32,
}

impl Default for TypeScale {
    fn default() -> Self {
        TypeScale {
            xs: 11.0,
            sm: 12.0,
            base: 14.0,
            md: 16.0,
            lg: 18.0,
            h3: 22.0,
            h2: 28.0,
            h1: 34.0,
        }
    }
}

/// Control heights (`--h-sm/md/lg/xl`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlHeights {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
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

/// Motion durations in seconds (`--dur-1/2/3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionDurations {
    pub fast: f32,
    pub base: f32,
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

/// Font weights the Forge sans family ships (see `theme::fonts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Medium,
    SemiBold,
}

pub const SIDEBAR_WIDTH: f32 = 240.0;
pub const SIDEBAR_RAIL: f32 = 56.0;
pub const TOPBAR_HEIGHT: f32 = 48.0;
pub const STATUSBAR_HEIGHT: f32 = 28.0;
