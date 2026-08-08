//! Theme behaviour: which role selects which token, how a custom accent
//! re-derives its dependants, and the frozen chart order.
//!
//! The palette's values are NOT asserted here. `theme/palette.rs` is generated
//! from `packages/tokens/tokens.source.mjs`. `just check` fails while the
//! committed palette does not match it. A literal hex in this file would only
//! be a second copy of the source. It would also turn every token change into
//! a two-file edit. The tests below thus assert relations instead.

use egui::Color32;
use forge_egui::theme::{chart_series, series_color, Scheme, Severity, Surface, TextRole, Theme};

#[test]
fn each_theme_declares_its_scheme() {
    assert_eq!(Theme::dark().scheme, Scheme::Dark);
    assert_eq!(Theme::light().scheme, Scheme::Light);
}

/// The named roles select the steps of the two ramps in order: `bg` rises from
/// the page to the popover, `fg` descends from primary text to disabled.
#[test]
fn roles_select_their_ramp_step() {
    let t = Theme::dark();
    let surfaces = [
        Surface::Page,
        Surface::Card,
        Surface::Hover,
        Surface::Pressed,
        Surface::Popover,
    ];
    for (i, role) in surfaces.into_iter().enumerate() {
        assert_eq!(t.surface(role), t.bg[i], "surface {role:?}");
    }
    let texts = [
        TextRole::Primary,
        TextRole::Secondary,
        TextRole::Tertiary,
        TextRole::Disabled,
    ];
    for (i, role) in texts.into_iter().enumerate() {
        assert_eq!(t.text(role), t.fg[i], "text {role:?}");
    }
}

/// forge-tui pre-composites its tints, because a terminal has no alpha channel.
/// Every `*-bg` tint here stays genuinely translucent instead. It thus
/// composites correctly over whatever surface a widget paints it on. Which
/// alpha each one carries is the token source's business, not this file's.
#[test]
fn tints_carry_real_alpha() {
    for t in [Theme::dark(), Theme::light()] {
        let tints = [
            t.accent.bg,
            t.success.bg,
            t.warning.bg,
            t.danger.bg,
            t.info.bg,
        ];
        for tint in tints {
            assert!(
                tint.a() > 0 && tint.a() < u8::MAX,
                "tint is not translucent: {tint:?} in {}",
                t.name
            );
        }
    }
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
    // Tint keeps the brand hue at the alpha the token source declares.
    assert_eq!(t.accent.bg.a(), Theme::dark().accent.bg.a());
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
    assert_eq!(series_color(&t, 5), t.text(TextRole::Tertiary));
    assert_eq!(series_color(&t, 99), t.text(TextRole::Tertiary));
}
