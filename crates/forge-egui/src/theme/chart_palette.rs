//! Data-viz series palette — mirrors `packages/charts/src/palette.ts`.
//!
//! The order is CVD-validated and load-bearing: adjacent series stay
//! distinguishable under the common color-vision deficiencies. NEVER reorder
//! or cycle it; series beyond the fifth fold into a `fg[2]` "Other" bucket.

use super::Theme;
use egui::Color32;

/// Number of distinct chart series before overflow folds into "Other".
pub const CHART_SERIES_LEN: usize = 5;

/// The fixed series palette: `[accent, danger, success, warning, info]`.
pub fn chart_series(theme: &Theme) -> [Color32; CHART_SERIES_LEN] {
    [
        theme.accent.base,
        theme.danger.base,
        theme.success.base,
        theme.warning.base,
        theme.info.base,
    ]
}

/// Color for series `i`; indices past the palette fold into the `fg[2]`
/// "Other" tone rather than cycling.
pub fn series_color(theme: &Theme, i: usize) -> Color32 {
    if i < CHART_SERIES_LEN {
        chart_series(theme)[i]
    } else {
        theme.fg[2]
    }
}
