//! # forge-egui
//!
//! The Forge design system for native desktop UIs: an egui widget kit, a
//! token-exact theme, and an optional app runtime — the egui sibling of
//! `forge-tui` (terminals) and `@forge/ui` (web).
//!
//! Widgets follow one shape everywhere: a builder struct plus `.show(ui)`,
//! returning a [`ForgeResponse`](response::ForgeResponse) whose
//! [`Outcome`](response::Outcome) mirrors the contract shared by every Forge
//! kit (`Ignored` / `Consumed` / `Changed` / `Submitted` / `Cancelled`).
//! Value-bound form widgets borrow the app's data (`Input::new(&mut text)`);
//! widgets with real internal state pair with an explicit `FooState` struct
//! owned by the app.
//!
//! The widgets work in any eframe app once [`Theme::apply`](theme::Theme::apply)
//! has installed the theme on the `egui::Context`. The [`runtime`] module adds
//! the full Forge app frame — `run()`, `Shell`, toasts, dialogs, particle FX —
//! but is never required.
//!
//! ```ignore
//! use forge_egui::prelude::*;
//!
//! struct Hello { name: String }
//!
//! impl forge_egui::runtime::App for Hello {
//!     fn ui(&mut self, ui: &mut egui::Ui, _ctx: &mut Ctx) {
//!         Input::new(&mut self.name).label("Name").show(ui);
//!     }
//! }
//!
//! fn main() -> forge_egui::Result<()> {
//!     forge_egui::runtime::run(
//!         Hello { name: String::new() },
//!         Theme::dark(),
//!         RunOptions::default(),
//!     )
//! }
//! ```

pub mod error;
pub mod response;
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
pub mod rt;
pub mod runtime;
pub mod theme;
pub mod widgets;

pub use error::{Error, Result};
pub use runtime::run;
// Top-level re-exports (also in the prelude) so consumers never pin their
// own, possibly diverging, egui/eframe versions.
pub use {eframe, egui};
// The block-editor document model, re-exported so consumers share one
// forge-blocks version with the widget.
#[cfg(feature = "blocks")]
pub use forge_blocks;

pub mod prelude {
    pub use crate::response::{ForgeResponse, Outcome};
    pub use crate::runtime::{
        run, App, Command, Ctx, DialogResult, NavItem, NavSection, RunOptions, Shell, ShellState,
        ToastHandle,
    };
    pub use crate::theme::{chart_series, series_color, Scheme, Severity, Theme};
    pub use crate::widgets::*;
    pub use crate::{Error, Result};
    // Re-exported so downstream apps never pin their own (possibly diverging)
    // egui/eframe versions.
    pub use eframe;
    pub use egui;
}
