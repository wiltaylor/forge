//! The ambient theme — the only theme a widget paints with.
//!
//! The ambient theme is one process-wide slot, so every test here writes shared
//! state. Two rules keep that safe. First, each test takes `AMBIENT_LOCK` and
//! installs the theme it wants, so no test depends on what ran before it.
//! Second, no other test binary swaps the ambient theme — `tests/*.rs` is one
//! binary per file, so these swaps cannot reach them.

use forge_tui::theme::{ambient_theme, set_ambient_theme, Theme};
use forge_tui::widgets::{paint, Spinner};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;
use std::sync::{Mutex, MutexGuard, PoisonError};

static AMBIENT_LOCK: Mutex<()> = Mutex::new(());

/// A failed assertion leaves the ambient theme swapped and poisons the lock;
/// neither matters, because every test installs its own theme first.
fn lock_ambient() -> MutexGuard<'static, ()> {
    AMBIENT_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Paint a spinner into a bare buffer and read back the glyph's colour. The
/// spinner paints its glyph in `accent.base`, which differs between the dark
/// and the light theme.
fn glyph_color(spinner: Spinner<'_>) -> Color {
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
    spinner.render(buf.area, &mut buf);
    buf[(0, 0)].fg
}

/// The bug this whole change exists to fix: a widget must follow a theme
/// switch, not keep the scheme it booted with.
///
/// The spinner is built once, before either swap, and painted twice. Building a
/// fresh one after each swap would pass even if the theme were captured at
/// build time — which is the bug.
#[test]
fn switching_the_ambient_theme_repaints_a_widget() {
    let _guard = lock_ambient();
    let spinner = Spinner::new();

    set_ambient_theme(Theme::dark());
    assert_eq!(glyph_color(spinner.clone()), Theme::dark().accent.base);

    set_ambient_theme(Theme::light());
    assert_eq!(glyph_color(spinner), Theme::light().accent.base);
}

#[test]
fn set_ambient_theme_returns_the_theme_it_replaced() {
    let _guard = lock_ambient();
    set_ambient_theme(Theme::light());

    assert_eq!(set_ambient_theme(Theme::dark()), Theme::light());
    assert_eq!(ambient_theme(), Theme::dark());
}

/// `paint` is the whole widget-side protocol, so the empty-area guard it holds
/// is the one every widget relies on. An empty area must paint nothing — and
/// must not read the theme, since there is nothing to paint it with.
#[test]
fn paint_skips_an_area_with_no_cells() {
    let _guard = lock_ambient();
    set_ambient_theme(Theme::light());

    let mut painted = false;
    paint(Rect::new(0, 0, 0, 4), |_| painted = true);
    paint(Rect::new(0, 0, 4, 0), |_| painted = true);
    assert!(!painted);

    paint(Rect::new(0, 0, 4, 1), |t| {
        painted = true;
        assert_eq!(*t, Theme::light());
    });
    assert!(painted);
}
