//! Color math: hex helpers, alpha blending, and lightness shifts. The
//! terminal-only machinery from forge-tui (`ColorMode`, xterm quantization)
//! has no counterpart here — egui paints truecolor with real alpha.

use egui::Color32;

/// Build an opaque color from a `0xRRGGBB` literal: `rgb(0x0B0D10)`.
pub const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// The color with its alpha replaced (unmultiplied 0–255).
pub fn with_alpha(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// sRGB alpha composite of `fg` at `alpha` over `bg` — for the rare cases a
/// widget needs a pre-flattened tint instead of painting with real alpha.
pub fn blend(fg: Color32, bg: Color32, alpha: f32) -> Color32 {
    let mix = |f: u8, b: u8| -> u8 {
        (f as f32 * alpha + b as f32 * (1.0 - alpha))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        mix(fg.r(), bg.r()),
        mix(fg.g(), bg.g()),
        mix(fg.b(), bg.b()),
    )
}

/// Lighten (`amount` > 0) or darken (`amount` < 0) by mixing toward
/// white/black. Used to derive hover/press variants from a custom accent.
pub fn shift(c: Color32, amount: f32) -> Color32 {
    if amount >= 0.0 {
        blend(Color32::WHITE, c, amount.min(1.0))
    } else {
        blend(Color32::BLACK, c, (-amount).min(1.0))
    }
}
