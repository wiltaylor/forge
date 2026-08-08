//! forge-xterm — one home for the terminal wire encodings.
//!
//! The kits (forge-tui, forge-egui) each drive a terminal emulator and must
//! turn user input into the bytes xterm-compatible programs expect. Those
//! bytes are a protocol, not a rendering choice, so they live here once
//! instead of in each kit.
//!
//! The crate is synchronous and has no dependencies. forge-tui has no async
//! runtime and does not want one; keeping this crate free of dependencies is
//! what lets both kits share the encodings without the terminal engines
//! having to merge.
//!
//! Each kit adapts its own input types (crossterm's, egui's) to the vocabulary
//! here — see [`mouse::MouseReport`]. The authored corpus in `tests/corpus.rs`
//! pins every event, modifier and mode to its exact bytes, so a case added for
//! one kit is checked for both.

pub mod mouse;
