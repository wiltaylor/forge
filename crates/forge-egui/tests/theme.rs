//! Token exactness: the palette must match `packages/tokens/css/tokens.css`
//! (via forge-tui's converted constants) literally — these values are the
//! design system's contract.

use egui::Color32;
use forge_egui::theme::{chart_series, series_color, Scheme, Severity, Theme};

fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

#[test]
fn dark_palette_is_token_exact() {
    let t = Theme::dark();
    assert_eq!(t.scheme, Scheme::Dark);
    assert_eq!(hex(t.bg[0]), "#0B0D10");
    assert_eq!(hex(t.bg[1]), "#11141A");
    assert_eq!(hex(t.bg[2]), "#171B22");
    assert_eq!(hex(t.bg[3]), "#1E232C");
    assert_eq!(hex(t.bg[4]), "#252B36");
    assert_eq!(hex(t.fg[0]), "#ECEEF2");
    assert_eq!(hex(t.fg[3]), "#4E5664");
    assert_eq!(hex(t.border.subtle), "#1A1F27");
    assert_eq!(hex(t.border.default), "#262C36");
    assert_eq!(hex(t.border.strong), "#3A4250");
    // The true browser-rendered accent — NOT the #5A8FDB fallback stand-in.
    assert_eq!(hex(t.accent.base), "#2389E2");
    assert_eq!(hex(t.accent.hover), "#2896F5");
    assert_eq!(hex(t.accent.press), "#0077CC");
    assert_eq!(hex(t.accent.fg), "#95C9FF");
    assert_eq!(hex(t.success.base), "#4EB068");
    assert_eq!(hex(t.warning.base), "#EBA941");
    assert_eq!(hex(t.danger.base), "#F14D4C");
    assert_eq!(hex(t.info.base), "#1CA6D9");
}

#[test]
fn light_palette_is_token_exact() {
    let t = Theme::light();
    assert_eq!(t.scheme, Scheme::Light);
    assert_eq!(hex(t.bg[0]), "#FAFAFA");
    assert_eq!(hex(t.fg[0]), "#0C0F14");
    assert_eq!(hex(t.accent.base), "#006BB9");
    assert_eq!(hex(t.danger.base), "#C6001F");
}

#[test]
fn tints_carry_real_alpha() {
    let t = Theme::dark();
    // 14% tints — unlike forge-tui these are NOT pre-blended over bg1.
    assert_eq!(t.accent.bg.a(), 36);
    assert_eq!(t.success.bg.a(), 36);
    assert_eq!(t.danger.bg.a(), 36);
    // Light warning is the web's 20% tint.
    assert_eq!(Theme::light().warning.bg.a(), 51);
}

#[test]
fn with_accent_derives_interaction_states() {
    let brand = Color32::from_rgb(0x8A, 0x63, 0xD2);
    let t = Theme::dark().with_accent(brand);
    assert_eq!(t.accent.base, brand);
    assert_ne!(t.accent.hover, brand);
    assert_ne!(t.accent.press, brand);
    // Dark scheme: hover lightens, press darkens.
    assert!(t.accent.hover.r() > brand.r());
    assert!(t.accent.press.r() < brand.r());
    // Tint keeps the brand hue with the standard alpha.
    assert_eq!(t.accent.bg.a(), 36);
    // Everything else untouched.
    assert_eq!(t.bg, Theme::dark().bg);
}

#[test]
fn severity_selects_the_matching_triple() {
    let t = Theme::dark();
    assert_eq!(t.severity(Severity::Danger).base, t.danger.base);
    assert_eq!(t.severity(Severity::Success).fg, t.success.fg);
}

#[test]
fn chart_palette_order_is_locked() {
    let t = Theme::dark();
    let series = chart_series(&t);
    assert_eq!(
        series,
        [
            t.accent.base,
            t.danger.base,
            t.success.base,
            t.warning.base,
            t.info.base
        ]
    );
    // Overflow folds into "Other" — never cycles.
    assert_eq!(series_color(&t, 5), t.fg[2]);
    assert_eq!(series_color(&t, 99), t.fg[2]);
}
