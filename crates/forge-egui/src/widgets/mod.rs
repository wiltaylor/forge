//! The Forge widget catalog. Families mirror `forge-tui/src/widgets` and
//! `@forge/ui`; everything is re-exported flat so `use forge_egui::widgets::*`
//! (or the prelude) brings the whole kit in.
//!
//! Widgets read their tokens from the theme installed on the context
//! ([`Theme::of`](crate::theme::Theme::of)) — install one with
//! [`Theme::apply`](crate::theme::Theme::apply) before showing any of them.

pub mod charts;
pub mod data;
#[cfg(feature = "calendar")]
pub mod date;
#[cfg(any(feature = "vnc", feature = "rdp"))]
pub mod desktop;
pub mod feedback;
pub mod forms;
pub mod overlays;
pub mod primitives;
pub mod specialty;
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
pub(crate) mod stream;
pub mod structure;
#[cfg(feature = "term")]
pub mod term;
pub(crate) mod util;

pub use charts::*;
pub use data::*;
#[cfg(feature = "calendar")]
pub use date::*;
#[cfg(any(feature = "vnc", feature = "rdp"))]
pub use desktop::*;
pub use feedback::*;
pub use forms::*;
pub use overlays::*;
pub use primitives::*;
pub use specialty::*;
pub use structure::*;
#[cfg(feature = "term")]
pub use term::*;

use crate::theme::Theme;
use egui::Color32;

// Families land milestone by milestone:
// pub mod feedback;     M4
// pub mod overlays;     M4
// pub mod structure;    M5
// pub mod data;         M6
// pub mod charts;       M7
// pub mod date;         M7  [calendar]
// pub mod specialty;    M8  [markdown/chat/code] + flowchart/node_graph
// stream/term/desktop   W1–W4 [term/vnc/rdp]

/// Semantic tone selector shared by Badge, StatusDot, Stat deltas, Alert, …
/// Mirrors `@forge/ui`'s `Tone` type (`Severity` plus `Neutral`/`Accent`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
    Info,
}

impl Tone {
    /// `(base, tint-bg, text-on-tint)` for this tone.
    pub fn triple(self, t: &Theme) -> (Color32, Color32, Color32) {
        match self {
            Tone::Neutral => (t.fg[2], t.bg[3], t.fg[1]),
            Tone::Accent => (t.accent.base, t.accent.bg, t.accent.fg),
            Tone::Success => (t.success.base, t.success.bg, t.success.fg),
            Tone::Warning => (t.warning.base, t.warning.bg, t.warning.fg),
            Tone::Danger => (t.danger.base, t.danger.bg, t.danger.fg),
            Tone::Info => (t.info.base, t.info.bg, t.info.fg),
        }
    }
}
