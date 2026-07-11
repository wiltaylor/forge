//! Color math: hex helpers, alpha pre-blending, and terminal color degradation.
//!
//! Terminals have no alpha channel, so the web tokens' translucent tints are
//! pre-composited over a concrete surface at palette-build time via [`blend`].
//! Truecolor themes degrade to 256-color via [`to_indexed`]; 16-color
//! terminals get a hand-written semantic mapping in `Theme::quantized`.

use ratatui::style::Color;

/// How many colors the terminal can address. Detected once at startup and
/// used to quantize the [`Theme`](super::Theme) once — never per cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Indexed256,
    Ansi16,
}

impl ColorMode {
    /// Detect from the environment. `FORGE_TUI_COLOR` (`truecolor`/`256`/`16`)
    /// overrides; otherwise `COLORTERM=truecolor|24bit` selects truecolor,
    /// `TERM=linux|dumb` falls to 16 colors, and everything else gets 256 —
    /// the safe default for multiplexers and ssh sessions that strip
    /// `COLORTERM`.
    pub fn detect() -> ColorMode {
        if let Ok(v) = std::env::var("FORGE_TUI_COLOR") {
            match v.to_ascii_lowercase().as_str() {
                "truecolor" | "24bit" | "rgb" => return ColorMode::TrueColor,
                "256" | "indexed" => return ColorMode::Indexed256,
                "16" | "ansi" => return ColorMode::Ansi16,
                _ => {}
            }
        }
        if let Ok(v) = std::env::var("COLORTERM") {
            let v = v.to_ascii_lowercase();
            if v.contains("truecolor") || v.contains("24bit") {
                return ColorMode::TrueColor;
            }
        }
        match std::env::var("TERM").as_deref() {
            Ok("linux") | Ok("dumb") | Err(_) => ColorMode::Ansi16,
            Ok(t) if t.contains("256color") => ColorMode::Indexed256,
            Ok(_) => ColorMode::Indexed256,
        }
    }
}

/// Build a color from a `0xRRGGBB` literal: `rgb(0x0B0D10)`.
pub const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn channels(c: Color) -> Option<(u8, u8, u8)> {
    match c {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        _ => None,
    }
}

/// sRGB alpha composite of `fg` at `alpha` over `bg` — what a browser does
/// with `oklch(... / 0.14)` painted on a surface. Non-RGB inputs pass through
/// unchanged.
pub fn blend(fg: Color, bg: Color, alpha: f32) -> Color {
    let (Some((fr, fg_, fb)), Some((br, bg_, bb))) = (channels(fg), channels(bg)) else {
        return fg;
    };
    let mix = |f: u8, b: u8| -> u8 {
        (f as f32 * alpha + b as f32 * (1.0 - alpha))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::Rgb(mix(fr, br), mix(fg_, bg_), mix(fb, bb))
}

/// Lighten (`amount` > 0) or darken (`amount` < 0) an RGB color by mixing
/// toward white/black. Used to derive hover/press variants from a custom
/// accent.
pub fn shift(c: Color, amount: f32) -> Color {
    if amount >= 0.0 {
        blend(Color::Rgb(255, 255, 255), c, amount.min(1.0))
    } else {
        blend(Color::Rgb(0, 0, 0), c, (-amount).min(1.0))
    }
}

/// xterm 256-color cube levels (indices 16..=231).
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

fn nearest_cube_level(v: u8) -> usize {
    let mut best = 0;
    let mut best_d = i32::MAX;
    for (i, &l) in CUBE.iter().enumerate() {
        let d = (v as i32 - l as i32).abs();
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

fn dist2(a: (u8, u8, u8), b: (u8, u8, u8)) -> i64 {
    let dr = a.0 as i64 - b.0 as i64;
    let dg = a.1 as i64 - b.1 as i64;
    let db = a.2 as i64 - b.2 as i64;
    dr * dr + dg * dg + db * db
}

/// Nearest xterm-256 index for an RGB triple, considering BOTH the 6×6×6
/// color cube and the 24-step grayscale ramp (232..=255). The ramp is what
/// keeps the five near-black Forge backgrounds distinct — cube-only
/// quantization collapses them all into level 0.
pub fn nearest_indexed(r: u8, g: u8, b: u8) -> u8 {
    let target = (r, g, b);
    // Cube candidate.
    let (ci, cj, ck) = (
        nearest_cube_level(r),
        nearest_cube_level(g),
        nearest_cube_level(b),
    );
    let cube_rgb = (CUBE[ci], CUBE[cj], CUBE[ck]);
    let cube_idx = 16 + 36 * ci + 6 * cj + ck;
    // Grayscale candidate: ramp value 8 + 10*i for i in 0..24.
    let avg = (r as i32 + g as i32 + b as i32) / 3;
    let gi = ((avg - 8 + 5) / 10).clamp(0, 23) as usize;
    let gray = (8 + 10 * gi) as u8;
    let gray_rgb = (gray, gray, gray);
    let gray_idx = 232 + gi;
    if dist2(target, gray_rgb) < dist2(target, cube_rgb) {
        gray_idx as u8
    } else {
        cube_idx as u8
    }
}

/// RGB value of an xterm-256 palette index (16..=255).
pub(crate) fn xterm_rgb(idx: u8) -> (u8, u8, u8) {
    if idx >= 232 {
        let g = 8 + 10 * (idx - 232);
        (g, g, g)
    } else {
        let i = idx - 16;
        (
            CUBE[(i / 36) as usize],
            CUBE[((i / 6) % 6) as usize],
            CUBE[(i % 6) as usize],
        )
    }
}

/// Nearest xterm-256 index skipping `used` — exhaustive over cube + ramp.
/// This is how the five near-black Forge backgrounds keep distinct indices
/// even when several share the same nearest gray.
pub fn nearest_indexed_excluding(r: u8, g: u8, b: u8, used: &[u8]) -> u8 {
    let mut best = 16u8;
    let mut best_d = i64::MAX;
    for idx in 16..=255u8 {
        if used.contains(&idx) {
            continue;
        }
        let d = dist2((r, g, b), xterm_rgb(idx));
        if d < best_d {
            best_d = d;
            best = idx;
        }
    }
    best
}

/// Recover an approximate RGB triple from a color: `Rgb` passes through,
/// `Indexed(16..=255)` goes through the xterm palette, everything else
/// (named ANSI-16 colors, `Reset`) has no portable RGB value.
pub(crate) fn approx_rgb(c: Color) -> Option<(u8, u8, u8)> {
    match c {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(i) if i >= 16 => Some(xterm_rgb(i)),
        _ => None,
    }
}

/// Quantize a single color for the given mode. RGB→indexed for 256-color
/// mode; identity otherwise (Ansi16 themes are built semantically, not by
/// numeric quantization).
pub fn quantize(c: Color, mode: ColorMode) -> Color {
    match (mode, c) {
        (ColorMode::Indexed256, Color::Rgb(r, g, b)) => Color::Indexed(nearest_indexed(r, g, b)),
        _ => c,
    }
}
