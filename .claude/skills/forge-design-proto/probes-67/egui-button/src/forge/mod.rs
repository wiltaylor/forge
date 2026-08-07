//! Forge for egui.
//!
//! Written from the `forge-design` skill in `.claude/skills/forge-design`:
//! `reference/laws.md`, `reference/anti-patterns.md`, `reference/egui.md`,
//! `reference/tokens.md`, `reference/controls/button.md` and
//! `reference/impl/egui/button.md`.
//!
//! The kit is written as a reusable module, so a binary crate flags the tokens and the
//! setters this one screen does not call.
#![allow(dead_code, unused_imports)]

pub mod button;
pub mod response;
pub mod theme;

pub use button::{Button, Variant};
pub use response::{ForgeResponse, Outcome};
pub use theme::{FontWeight, Severity, Theme, ThemeMode};
