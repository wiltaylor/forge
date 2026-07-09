//! Interactive remote-access widgets: PTY terminal, VNC and RDP viewers.
//!
//! Rust-only extensions behind opt-in cargo features (`term`, `term-ssh`,
//! `vnc`, `rdp`, or all via `widgets`) — NOT part of the frozen v1.0 API
//! contract. Each widget is a dedicated per-connection WebSocket under the
//! protected router group, enabled at runtime by a `with_*()` builder or env
//! flag. `/api/term` hands authenticated users a real shell (RCE by design);
//! VNC/RDP open outbound connections. Trusted dev contexts only — see
//! docs/widgets-protocol.md.

pub mod proto;

/// Capacity of the bounded per-connection channels bridging protocol tasks to
/// the WebSocket writer. Backpressure, not unbounded buffering, on slow
/// clients.
pub const CHANNEL_CAP: usize = 256;
