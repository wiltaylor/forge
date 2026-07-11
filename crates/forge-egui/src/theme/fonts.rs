//! Forge fonts: IBM Plex Sans (UI) and JetBrains Mono (code), embedded under
//! the `fonts` feature (SIL OFL 1.1 — license texts in `LICENSES/`).
//!
//! egui allows one weight per family, so Medium and SemiBold are registered
//! as *named* families; [`Theme::font`](super::Theme::font) picks the right
//! one from a [`FontWeight`](super::FontWeight). Without the feature all
//! weights collapse onto egui's default proportional font.

use super::tokens::FontWeight;
use egui::FontFamily;

#[cfg_attr(not(feature = "fonts"), allow(dead_code))]
pub(crate) const MEDIUM: &str = "plex-sans-medium";
#[cfg_attr(not(feature = "fonts"), allow(dead_code))]
pub(crate) const SEMIBOLD: &str = "plex-sans-semibold";
#[cfg_attr(not(feature = "fonts"), allow(dead_code))]
pub(crate) const MONO_BOLD: &str = "jetbrains-mono-bold";

/// Whether `family` is bound on this context's current fonts. Named families
/// are unbound until the first frame after [`install`] queues them.
pub(crate) fn bound(ctx: &egui::Context, family: &FontFamily) -> bool {
    ctx.fonts(|f| f.definitions().families.contains_key(family))
}

/// The [`FontFamily`] carrying the requested weight.
pub(crate) fn family(weight: FontWeight) -> FontFamily {
    #[cfg(feature = "fonts")]
    {
        match weight {
            FontWeight::Regular => FontFamily::Proportional,
            FontWeight::Medium => FontFamily::Name(MEDIUM.into()),
            FontWeight::SemiBold => FontFamily::Name(SEMIBOLD.into()),
        }
    }
    #[cfg(not(feature = "fonts"))]
    {
        let _ = weight;
        FontFamily::Proportional
    }
}

/// Install the Forge fonts on the context: Plex Sans Regular heads the
/// proportional list, JetBrains Mono heads the monospace list, and the
/// Medium/SemiBold/MonoBold weights become named families that fall back to
/// the egui defaults for glyph coverage (symbols, emoji).
#[cfg(feature = "fonts")]
pub(crate) fn install(ctx: &egui::Context) {
    use egui::FontData;
    use std::sync::Arc;

    let mut fonts = egui::FontDefinitions::default();

    let data: [(&str, &[u8]); 5] = [
        (
            "plex-sans",
            include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"),
        ),
        (
            MEDIUM,
            include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf"),
        ),
        (
            SEMIBOLD,
            include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
        ),
        (
            "jetbrains-mono",
            include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
        ),
        (
            MONO_BOLD,
            include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
        ),
    ];
    for (name, bytes) in data {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    // Default fallback chains (egui's built-ins provide symbol/emoji coverage).
    let prop_fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mono_fallback = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // Plex's symbol coverage is thin (no geometric shapes/chevrons); JetBrains
    // Mono and egui's bundled Hack fill the Glyph set for proportional text.
    let symbol_tail = ["jetbrains-mono".to_owned(), "Hack".to_owned()];

    let mut chain = |family: FontFamily, head: &str, fallback: &[String], tail: &[String]| {
        let mut list = vec![head.to_owned()];
        list.extend(fallback.iter().cloned());
        list.extend(tail.iter().cloned());
        fonts.families.insert(family, list);
    };

    chain(
        FontFamily::Proportional,
        "plex-sans",
        &prop_fallback,
        &symbol_tail,
    );
    chain(
        FontFamily::Name(MEDIUM.into()),
        MEDIUM,
        &prop_fallback,
        &symbol_tail,
    );
    chain(
        FontFamily::Name(SEMIBOLD.into()),
        SEMIBOLD,
        &prop_fallback,
        &symbol_tail,
    );
    chain(FontFamily::Monospace, "jetbrains-mono", &mono_fallback, &[]);
    chain(
        FontFamily::Name(MONO_BOLD.into()),
        MONO_BOLD,
        &mono_fallback,
        &[],
    );

    ctx.set_fonts(fonts);
}

#[cfg(not(feature = "fonts"))]
pub(crate) fn install(_ctx: &egui::Context) {}
