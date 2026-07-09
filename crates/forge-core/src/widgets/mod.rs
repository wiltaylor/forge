//! Interactive remote-access widgets: PTY terminal, VNC and RDP viewers.
//!
//! Rust-only extensions behind opt-in cargo features (`term`, `term-ssh`,
//! `vnc`, `rdp`, or all via `widgets`) — NOT part of the frozen v1.0 API
//! contract. The session engines here are transport-agnostic: they pump a
//! [`WidgetStream`] (JSON text frames for control, binary frames for
//! payload) and are driven by forge-server over per-connection WebSockets
//! and by forge-tauri over Tauri IPC channels. `/api/term` hands
//! authenticated users a real shell (RCE by design); VNC/RDP open outbound
//! connections. Trusted dev contexts only — see docs/widgets-protocol.md.

#[cfg(any(feature = "vnc", feature = "rdp"))]
pub mod keymap;
pub mod proto;
#[cfg(feature = "rdp")]
pub mod rdp;
#[cfg(feature = "term")]
pub mod term;
#[cfg(feature = "vnc")]
pub mod vnc;

use std::future::Future;

/// Capacity of the bounded per-connection channels bridging protocol tasks to
/// the transport writer. Backpressure, not unbounded buffering, on slow
/// clients.
pub const CHANNEL_CAP: usize = 256;

/// One frame on a widget connection, mirroring the WebSocket frame kinds the
/// protocol was designed around: JSON text = control, binary = payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetMsg {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

/// The peer is gone; the session should wind down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamClosed;

impl std::fmt::Display for StreamClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("widget stream closed")
    }
}

impl std::error::Error for StreamClosed {}

/// A bidirectional widget connection. forge-server implements this for the
/// axum WebSocket; forge-tauri for a (Channel out, mpsc in) pair.
///
/// `recv` yields only meaningful frames (transports handle ping/pong
/// themselves) and `None` once the peer is gone. Explicit RPITIT + `Send` so
/// generic session futures stay spawnable.
pub trait WidgetStream: Send {
    fn recv(&mut self) -> impl Future<Output = Option<WidgetMsg>> + Send;
    fn send(&mut self, msg: WidgetMsg) -> impl Future<Output = Result<(), StreamClosed>> + Send;
}

/// Runtime configuration for the terminal widget.
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

/// Runtime configuration for the VNC and RDP desktop widgets.
#[cfg(any(feature = "vnc", feature = "rdp"))]
#[derive(Debug, Clone, Default)]
pub struct DesktopConfig {
    /// Hosts outbound connections may target. `None` = any host.
    pub allow_hosts: Option<Vec<String>>,
}
