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
#[cfg(feature = "rdp")]
pub mod rdp;
#[cfg(feature = "term")]
pub mod term;
#[cfg(feature = "vnc")]
pub mod vnc;

/// Capacity of the bounded per-connection channels bridging protocol tasks to
/// the WebSocket writer. Backpressure, not unbounded buffering, on slow
/// clients.
pub const CHANNEL_CAP: usize = 256;

/// Runtime configuration for `/api/term` (set via
/// [`crate::ForgeApp::with_term_config`]).
#[cfg(feature = "term")]
#[derive(Debug, Clone)]
pub struct TermConfig {
    /// Shell for local sessions. `None` = `$SHELL`, falling back to `/bin/sh`.
    pub shell: Option<String>,
    /// Permit `mode: "local"` sessions (a real shell as the server uid).
    pub allow_local: bool,
    /// Permit `mode: "ssh"` sessions (requires the `term-ssh` feature).
    pub allow_ssh: bool,
    /// Hosts SSH sessions may target. `None` = any host.
    pub allow_hosts: Option<Vec<String>>,
}

#[cfg(feature = "term")]
impl Default for TermConfig {
    fn default() -> Self {
        Self {
            shell: None,
            allow_local: true,
            allow_ssh: true,
            allow_hosts: None,
        }
    }
}

/// Runtime configuration for `/api/desktop/vnc` and `/api/desktop/rdp`
/// (set via [`crate::ForgeApp::with_vnc_config`] / `with_rdp_config`).
#[cfg(any(feature = "vnc", feature = "rdp"))]
#[derive(Debug, Clone, Default)]
pub struct DesktopConfig {
    /// Hosts outbound connections may target. `None` = any host.
    pub allow_hosts: Option<Vec<String>>,
}
