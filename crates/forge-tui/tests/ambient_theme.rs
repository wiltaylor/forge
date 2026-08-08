//! The ambient theme — the theme a widget paints with when the caller gave it
//! none.
//!
//! The ambient theme is one process-wide slot, so every test here writes shared
//! state. Two rules keep that safe. First, each test takes `AMBIENT_LOCK` and
//! installs the theme it wants, so no test depends on what ran before it.
//! Second, all of the swapping lives in this file: `tests/*.rs` is one binary
//! per file, so no other test can see it.

use forge_tui::theme::{ambient_theme, resolve_theme, set_ambient_theme, Theme};
use forge_tui::widgets::Spinner;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;
use std::borrow::Cow;
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

/// The bug this whole change exists to fix: a widget built without a theme
/// must follow a theme switch, not keep the scheme it booted with.
#[test]
fn switching_the_ambient_theme_repaints_a_widget_that_carries_none() {
    let _guard = lock_ambient();
    set_ambient_theme(Theme::dark());
    assert_eq!(glyph_color(Spinner::new()), Theme::dark().accent.base);

    set_ambient_theme(Theme::light());
    assert_eq!(glyph_color(Spinner::new()), Theme::light().accent.base);
}

#[test]
fn an_explicit_theme_still_wins_over_the_ambient_one() {
    let _guard = lock_ambient();
    set_ambient_theme(Theme::light());

    let dark = Theme::dark();
    assert_eq!(glyph_color(Spinner::new().theme(&dark)), dark.accent.base);
}

#[test]
fn set_ambient_theme_returns_the_theme_it_replaced() {
    let _guard = lock_ambient();
    set_ambient_theme(Theme::light());

    assert_eq!(set_ambient_theme(Theme::dark()), Theme::light());
    assert_eq!(ambient_theme(), Theme::dark());
}

/// An explicit theme is borrowed, so resolving one costs nothing; the ambient
/// theme is snapshotted, so a swap mid-frame cannot change it under the paint.
#[test]
fn resolve_theme_borrows_an_explicit_theme_and_snapshots_the_ambient_one() {
    let _guard = lock_ambient();
    set_ambient_theme(Theme::light());

    let dark = Theme::dark();
    assert!(matches!(resolve_theme(Some(&dark)), Cow::Borrowed(_)));
    assert!(matches!(resolve_theme(None), Cow::Owned(_)));
    assert_eq!(*resolve_theme(None), Theme::light());
}
