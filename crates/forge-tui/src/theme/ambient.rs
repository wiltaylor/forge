//! The ambient theme: the theme a widget paints with when the caller gave it
//! none.
//!
//! One process-wide slot, swappable at any time. The runtime installs the
//! quantized theme into it at startup ([`runtime::run`](crate::runtime::run)),
//! and an app that offers a theme toggle installs the new theme the same way.
//! Widgets read it during render — through [`resolve_theme`] — so a swap shows
//! up on the next frame.
//!
//! This is the egui kit's shape, where `Theme::of(ctx)` reads the theme the app
//! installed on the context. A ratatui `Widget::render` gets no context, so the
//! slot is a static instead.
//!
//! ```
//! use forge_tui::theme::{ambient_theme, set_ambient_theme, Theme};
//! let previous = set_ambient_theme(Theme::light());
//! assert_eq!(ambient_theme().scheme, forge_tui::theme::Scheme::Light);
//! set_ambient_theme(previous);
//! ```

use super::Theme;
use std::borrow::Cow;
use std::mem;
use std::sync::{PoisonError, RwLock};

/// Dark until something installs another theme.
///
/// A poisoned lock is recovered rather than propagated wherever this is read or
/// written: the slot holds one value with no invariant across a write, so a
/// panicking painter cannot leave it half-updated — and taking the UI down over
/// a colour would be worse than carrying on.
static AMBIENT: RwLock<Theme> = RwLock::new(Theme::dark());

/// A snapshot of the ambient theme.
///
/// Callers get a clone rather than a borrow, so holding the result cannot block
/// a concurrent [`set_ambient_theme`] — and so a swap part-way through a frame
/// cannot change a theme a widget is already painting with.
pub fn ambient_theme() -> Theme {
    AMBIENT
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Install `theme` as the ambient theme and return the one it replaced.
///
/// Widgets pick it up on the next render. Call it once at startup — the runtime
/// does — and again on every theme switch.
pub fn set_ambient_theme(theme: Theme) -> Theme {
    let mut slot = AMBIENT.write().unwrap_or_else(PoisonError::into_inner);
    mem::replace(&mut slot, theme)
}

/// The theme for one render: `explicit` if the caller gave one, otherwise a
/// snapshot of the ambient theme.
///
/// This is the widget-side entry point. Deref the result to get the `&Theme` a
/// paint block wants:
///
/// ```
/// # use forge_tui::theme::{resolve_theme, Theme};
/// # let explicit: Option<&Theme> = None;
/// let t = &*resolve_theme(explicit);
/// let _ = t.accent.base;
/// ```
pub fn resolve_theme(explicit: Option<&Theme>) -> Cow<'_, Theme> {
    match explicit {
        Some(theme) => Cow::Borrowed(theme),
        None => Cow::Owned(ambient_theme()),
    }
}
