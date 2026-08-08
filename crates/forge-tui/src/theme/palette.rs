//! GENERATED FILE — do not edit by hand.
//! Source:     packages/tokens/tokens.source.mjs
//! Regenerate: just generate   (`just check` fails while this file is stale)
//!
//! Forge token palette as compile-time constants.
//!
//! The neutral ramps are the sRGB literals the source authors. The accent
//! and semantic tokens are authored in OKLCH. The generator converts them,
//! and each states the expression it came from.
//!
//! Note that `packages/term/src/theme.ts` carries `#5A8FDB` as the accent
//! *fallback*. That is a hand-picked stand-in for when CSS resolution
//! fails, and deliberately not the token value. These values are what
//! browsers actually paint.
//!
//! Terminals have no alpha. Each `*-bg` tint thus arrives pre-composited
//! over the surface its token names — the card it usually sits on. A tint
//! painted on another surface will be marginally off. Call
//! [`blend`](super::color::blend) directly if that matters.

use super::color::rgb;
use super::{Accent, BorderTokens, Scheme, SemanticTriple, Theme};

pub const DARK: Theme = Theme {
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
        base: rgb(0x2389E2),     // oklch(0.62 0.16 250)
        hover: rgb(0x2896F5),    // oklch(0.66 0.17 250)
        press: rgb(0x0077CC),    // oklch(0.56 0.16 250)
        bg: rgb(0x142436),       // oklch(0.62 0.16 250 / 0.14) over bg-1
        fg: rgb(0x95C9FF),       // oklch(0.82 0.13 250)
        contrast: rgb(0xFFFFFF), // text on solid accent
    },
    success: SemanticTriple {
        base: rgb(0x4EB068), // oklch(0.68 0.14 150)
        bg: rgb(0x1A2A25),   // oklch(0.68 0.14 150 / 0.14) over bg-1
        fg: rgb(0x6DE18B),   // oklch(0.82 0.16 150)
    },
    warning: SemanticTriple {
        base: rgb(0xEBA941), // oklch(0.78 0.14 75)
        bg: rgb(0x30291F),   // oklch(0.78 0.14 75 / 0.14) over bg-1
        fg: rgb(0xFEC766),   // oklch(0.86 0.13 80)
    },
    danger: SemanticTriple {
        base: rgb(0xF14D4C), // oklch(0.65 0.20 25)
        bg: rgb(0x301C21),   // oklch(0.65 0.20 25 / 0.14) over bg-1
        fg: rgb(0xFF958D),   // oklch(0.78 0.16 25)
    },
    info: SemanticTriple {
        base: rgb(0x1CA6D9), // oklch(0.68 0.13 230)
        bg: rgb(0x132835),   // oklch(0.68 0.13 230 / 0.14) over bg-1
        fg: rgb(0x6FD2FF),   // oklch(0.82 0.12 230)
    },
};

pub const LIGHT: Theme = Theme {
    name: "forge-light",
    scheme: Scheme::Light,
    bg: [
        rgb(0xFAFAFA), // page
        rgb(0xFFFFFF), // card
        rgb(0xF4F5F7), // hover / nested card
        rgb(0xEAECEF), // pressed / active row
        rgb(0xFFFFFF), // popover, dropdown
    ],
    fg: [
        rgb(0x0C0F14), // primary text
        rgb(0x3D4654), // secondary text
        rgb(0x6B7383), // tertiary, captions
        rgb(0xA0A6B2), // disabled, placeholder
    ],
    border: BorderTokens {
        subtle: rgb(0xEEF0F3),
        default: rgb(0xDCDFE4),
        strong: rgb(0xB6BBC4),
    },
    accent: Accent {
        base: rgb(0x006BB9),     // oklch(0.52 0.18 250)
        hover: rgb(0x005A9D),    // oklch(0.46 0.19 250)
        press: rgb(0x004981),    // oklch(0.40 0.19 250)
        bg: rgb(0xDBECF7),       // oklch(0.55 0.17 250 / 0.14) over bg-1
        fg: rgb(0x004479),       // oklch(0.38 0.19 250)
        contrast: rgb(0xFFFFFF), // text on solid accent
    },
    success: SemanticTriple {
        base: rgb(0x007835), // oklch(0.50 0.15 150)
        bg: rgb(0xD7ECE0),   // oklch(0.55 0.15 150 / 0.16) over bg-1
        fg: rgb(0x004B1E),   // oklch(0.36 0.14 150)
    },
    warning: SemanticTriple {
        base: rgb(0xB97500), // oklch(0.62 0.16 70)
        bg: rgb(0xF3E5CC),   // oklch(0.65 0.16 70 / 0.20) over bg-1
        fg: rgb(0x6B3900),   // oklch(0.40 0.14 60)
    },
    danger: SemanticTriple {
        base: rgb(0xC6001F), // oklch(0.52 0.22 25)
        bg: rgb(0xF8DFE1),   // oklch(0.55 0.21 25 / 0.14) over bg-1
        fg: rgb(0x940015),   // oklch(0.42 0.20 25)
    },
    info: SemanticTriple {
        base: rgb(0x006D91), // oklch(0.50 0.14 230)
        bg: rgb(0xD6EAF1),   // oklch(0.55 0.14 230 / 0.16) over bg-1
        fg: rgb(0x00435B),   // oklch(0.36 0.13 230)
    },
};
