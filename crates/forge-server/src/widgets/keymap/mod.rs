//! Browser `KeyboardEvent` → remote-protocol key code tables (US layout v1).
//!
//! The desktop widget sends the layout-independent `code` plus, for
//! printables, the produced character in `key` (docs/widgets-protocol.md).

#[cfg(feature = "vnc")]
pub mod keysym;
#[cfg(feature = "rdp")]
pub mod scancode;
