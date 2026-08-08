use forge_tui::theme::{
    ambient_theme, blend, chart_series, color::nearest_indexed, ColorMode, Severity, Surface,
    TextRole, Theme,
};
use ratatui::style::Color;

/// Nothing in this test binary installs a theme, so this asserts the state a
/// widget paints in before the runtime gets a chance to. The swap tests live in
/// `ambient_theme.rs`, which is its own binary and cannot reach this one.
#[test]
fn the_ambient_theme_starts_dark() {
    assert_eq!(ambient_theme(), Theme::dark());
}

/// The five near-black dark backgrounds must stay distinct after 256-color
/// quantization — this is what the grayscale-ramp-aware quantizer is for.
#[test]
fn dark_bg_ramp_stays_distinct_in_256_colors() {
    let q = Theme::dark().quantized(ColorMode::Indexed256);
    let surfaces = [
        Surface::Page,
        Surface::Card,
        Surface::Hover,
        Surface::Pressed,
        Surface::Popover,
    ];
    let mut indices: Vec<u8> = surfaces
        .into_iter()
        .map(|s| match q.surface(s) {
            Color::Indexed(i) => i,
            other => panic!("expected indexed color for {s:?}, got {other:?}"),
        })
        .collect();
    indices.dedup();
    assert_eq!(indices.len(), 5, "bg ramp collapsed: {indices:?}");
}

/// The chart palette order is CVD-validated and frozen — mirrors
/// packages/charts/src/palette.ts. Do NOT "fix" this order.
#[test]
fn chart_series_order_is_locked() {
    let t = Theme::dark();
    assert_eq!(
        chart_series(&t),
        [
            t.accent.base,
            t.danger.base,
            t.success.base,
            t.warning.base,
            t.info.base,
        ]
    );
    // Overflow folds into the tertiary-text "Other" tone, never cycles.
    assert_eq!(
        forge_tui::theme::series_color(&t, 5),
        t.text(TextRole::Tertiary)
    );
    assert_eq!(
        forge_tui::theme::series_color(&t, 99),
        t.text(TextRole::Tertiary)
    );
}

#[test]
fn blend_composites_in_srgb() {
    let out = blend(Color::Rgb(255, 255, 255), Color::Rgb(0, 0, 0), 0.5);
    assert_eq!(out, Color::Rgb(128, 128, 128));
    let out = blend(Color::Rgb(100, 200, 50), Color::Rgb(100, 200, 50), 0.14);
    assert_eq!(out, Color::Rgb(100, 200, 50));
    // Non-RGB passes through.
    assert_eq!(blend(Color::Red, Color::Rgb(0, 0, 0), 0.5), Color::Red);
}

/// The dark `*-bg` tints are the token alpha (0.14) pre-blended over the
/// card surface.
#[test]
fn dark_tints_are_preblended_over_the_card_surface() {
    let t = Theme::dark();
    assert_eq!(
        t.accent.bg,
        blend(t.accent.base, t.surface(Surface::Card), 0.14)
    );
}

#[test]
fn ansi16_uses_semantic_mapping() {
    let q = Theme::dark().quantized(ColorMode::Ansi16);
    assert_eq!(q.danger.base, Color::Red);
    assert_eq!(q.success.base, Color::Green);
    assert_eq!(q.warning.base, Color::Yellow);
    assert_eq!(q.accent.base, Color::Blue);
    assert_eq!(q.info.base, Color::Cyan);
    // -fg variants map to the bright counterparts.
    assert_eq!(q.danger.fg, Color::LightRed);
    assert_eq!(q.accent.fg, Color::LightBlue);
    // Page background stays the terminal default.
    assert_eq!(q.surface(Surface::Page), Color::Reset);
}

#[test]
fn nearest_indexed_hits_exact_cube_and_ramp_points() {
    assert_eq!(nearest_indexed(0, 0, 0), 16); // cube origin
    assert_eq!(nearest_indexed(255, 255, 255), 231); // cube max
    assert_eq!(nearest_indexed(8, 8, 8), 232); // ramp start
    assert_eq!(nearest_indexed(238, 238, 238), 255); // ramp end
}

#[test]
fn severity_lookup_matches_fields() {
    let t = Theme::dark();
    assert_eq!(t.severity(Severity::Danger).base, t.danger.base);
    assert_eq!(t.severity(Severity::Info).fg, t.info.fg);
}

#[test]
fn with_accent_rederives_dependent_tokens() {
    let brand = Color::Rgb(0x8A, 0x63, 0xD2);
    let t = Theme::dark().with_accent(brand);
    assert_eq!(t.accent.base, brand);
    assert_ne!(t.accent.hover, brand);
    assert_eq!(t.accent.bg, blend(brand, t.surface(Surface::Card), 0.14));
}
