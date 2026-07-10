//! # forge-tui
//!
//! The Forge design system for terminal UIs: ratatui widgets with the same
//! dark-default, dense, technical aesthetic as the Forge web components, a
//! token-exact [`theme`], and an optional [`runtime`] (event loop, focus,
//! overlays, toasts) for full applications.
//!
//! Every widget is a plain ratatui `Widget`/`StatefulWidget` — usable in any
//! existing ratatui app without the runtime, just as the Forge web components
//! drop into any SolidJS app. Interaction follows one pattern: a `FooState`
//! owns the persistent state and returns an [`event::Outcome`] from
//! `handle_key`; `Outcome::Ignored` means "keep routing", like DOM bubbling.
//!
//! Pinned to ratatui 0.29; import crossterm types via `ratatui::crossterm`
//! so the versions can never diverge.
//!
//! ## Feature flags
//! Core widgets have zero optional dependencies. Heavier widgets are gated:
//! `markdown` (pulldown-cmark), `chat` (implies markdown), `code` (syntect),
//! `calendar` (time), `term` (portable-pty + vt100), or `full` for everything.

pub mod error;
pub mod event;
pub mod runtime;
pub mod text;
pub mod theme;
pub mod widgets;

pub use error::{Error, Result};

/// One-stop imports for building a forge-tui app.
pub mod prelude {
    pub use crate::event::{is_press, KeyCombo, Keymap, Outcome};
    pub use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    pub use crate::runtime::{
        run, App, AppShell, ConfirmDialog, Ctx, DialogResult, FocusId, FocusRing, HelpOverlay,
        MenuOverlay, NavSection, Overlay, OverlayOutcome, OverlayStack, OwnedCommand,
        OwnedMenuEntry, PaletteOverlay, RunOptions, ShellState, ToastHandle,
    };
    pub use crate::theme::{
        blend, chart_series, default_theme, series_color, set_default_theme, ColorMode, Scheme,
        Severity, Theme,
    };
    pub use crate::widgets::*;
}
