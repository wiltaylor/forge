//! Forge token palette. Neutral ramps are the literal hex values from
//! `packages/tokens/css/tokens.css`; accent/semantic tokens are authored in
//! OKLCH there and converted to sRGB (each annotated with its source).
//!
//! Unlike forge-tui — which pre-composites the translucent `*-bg` tints over
//! the card surface because terminals have no alpha — these tints carry REAL
//! alpha, exactly like the web (`oklch(... / 0.14)`), so they composite
//! correctly over any surface. In the dark scheme the web tint source is the
//! token's own base color; in the light scheme it is a slightly lighter OKLCH
//! variant (values below recovered from the web-rendered composites).

use super::color::{rgb, with_alpha};
use super::{Accent, BorderTokens, Scheme, SemanticTriple, Theme};

/// 14% alpha (0.14 × 255).
const A14: u8 = 36;
/// 16% alpha.
const A16: u8 = 41;
/// 20% alpha.
const A20: u8 = 51;

pub fn dark() -> Theme {
    Theme {
        name: "forge-dark",
        scheme: Scheme::Dark,
        bg: [
            rgb(0x0B0D10), // page
            rgb(0x11141A), // card
            rgb(0x171B22), // hover / nested card
            rgb(0x1E232C), // pressed / active row
            rgb(0x252B36), // popover, dropdown
        ],
        fg: [
            rgb(0xECEEF2), // primary text
            rgb(0xB7BDC8), // secondary text
            rgb(0x7C8593), // tertiary, captions
            rgb(0x4E5664), // disabled, placeholder
        ],
        border: BorderTokens {
            subtle: rgb(0x1A1F27),
            default: rgb(0x262C36),
            strong: rgb(0x3A4250),
        },
        accent: Accent {
            base: rgb(0x2389E2),                // oklch(0.62 0.16 250)
            hover: rgb(0x2896F5),               // oklch(0.66 0.17 250)
            press: rgb(0x0077CC),               // oklch(0.56 0.16 250)
            bg: with_alpha(rgb(0x2389E2), A14), // oklch(0.62 0.16 250 / 0.14)
            fg: rgb(0x95C9FF),                  // oklch(0.82 0.13 250)
            contrast: rgb(0xFFFFFF),            // text on solid accent
        },
        success: SemanticTriple {
            base: rgb(0x4EB068),                // oklch(0.68 0.14 150)
            bg: with_alpha(rgb(0x4EB068), A14), // oklch(0.68 0.14 150 / 0.14)
            fg: rgb(0x6DE18B),                  // oklch(0.82 0.16 150)
        },
        warning: SemanticTriple {
            base: rgb(0xEBA941),                // oklch(0.78 0.14 75)
            bg: with_alpha(rgb(0xEBA941), A14), // oklch(0.78 0.14 75 / 0.14)
            fg: rgb(0xFEC766),                  // oklch(0.86 0.13 80)
        },
        danger: SemanticTriple {
            base: rgb(0xF14D4C),                // oklch(0.65 0.20 25)
            bg: with_alpha(rgb(0xF14D4C), A14), // oklch(0.65 0.20 25 / 0.14)
            fg: rgb(0xFF958D),                  // oklch(0.78 0.16 25)
        },
        info: SemanticTriple {
            base: rgb(0x1CA6D9),                // oklch(0.68 0.13 230)
            bg: with_alpha(rgb(0x1CA6D9), A14), // oklch(0.68 0.13 230 / 0.14)
            fg: rgb(0x6FD2FF),                  // oklch(0.82 0.12 230)
        },
        radius: Default::default(),
        space: Default::default(),
        type_scale: Default::default(),
        control: Default::default(),
        motion: Default::default(),
    }
}

pub fn light() -> Theme {
    Theme {
        name: "forge-light",
        scheme: Scheme::Light,
        bg: [
            rgb(0xFAFAFA), // page
            rgb(0xFFFFFF), // card
            rgb(0xF4F5F7), // hover / nested card
            rgb(0xEAECEF), // pressed / active row
            rgb(0xFFFFFF), // popover, dropdown
        ],
        fg: [rgb(0x0C0F14), rgb(0x3D4654), rgb(0x6B7383), rgb(0xA0A6B2)],
        border: BorderTokens {
            subtle: rgb(0xEEF0F3),
            default: rgb(0xDCDFE4),
            strong: rgb(0xB6BBC4),
        },
        accent: Accent {
            base: rgb(0x006BB9),                // oklch(0.52 0.18 250)
            hover: rgb(0x005A9D),               // oklch(0.46 0.19 250)
            press: rgb(0x004981),               // oklch(0.40 0.19 250)
            bg: with_alpha(rgb(0x0077C6), A14), // oklch(0.55 0.17 250 / 0.14)
            fg: rgb(0x004479),                  // oklch(0.38 0.19 250)
            contrast: rgb(0xFFFFFF),
        },
        success: SemanticTriple {
            base: rgb(0x007835),                // oklch(0.50 0.15 150)
            bg: with_alpha(rgb(0x05883D), A16), // oklch(0.55 0.15 150 / 0.16)
            fg: rgb(0x004B1E),                  // oklch(0.36 0.14 150)
        },
        warning: SemanticTriple {
            base: rgb(0xB97500),                // oklch(0.62 0.16 70)
            bg: with_alpha(rgb(0xC37D00), A20), // oklch(0.65 0.16 70 / 0.20)
            fg: rgb(0x6B3900),                  // oklch(0.40 0.14 60)
        },
        danger: SemanticTriple {
            base: rgb(0xC6001F),                // oklch(0.52 0.22 25)
            bg: with_alpha(rgb(0xCD1A29), A14), // oklch(0.55 0.21 25 / 0.14)
            fg: rgb(0x940015),                  // oklch(0.42 0.20 25)
        },
        info: SemanticTriple {
            base: rgb(0x006D91),                // oklch(0.50 0.14 230)
            bg: with_alpha(rgb(0x007CA8), A16), // oklch(0.55 0.14 230 / 0.16)
            fg: rgb(0x00435B),                  // oklch(0.36 0.13 230)
        },
        radius: Default::default(),
        space: Default::default(),
        type_scale: Default::default(),
        control: Default::default(),
        motion: Default::default(),
    }
}
