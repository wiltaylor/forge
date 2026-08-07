//! Forge controls for ratatui, written from `.claude/skills/forge-design`.
//!
//! The crate is the namespace, so a control's type name is its catalogue name in
//! PascalCase with no prefix.
//!
//! This is a control kit inside a binary crate, so parts of the public surface
//! have no caller yet. That is the API, not dead code.
#![allow(dead_code)]

pub mod button;
pub mod outcome;
pub mod theme;

#[allow(unused_imports)]
pub use button::{Button, Size, Variant};
#[allow(unused_imports)]
pub use outcome::Outcome;
#[allow(unused_imports)]
pub use theme::{Severity, Theme};
