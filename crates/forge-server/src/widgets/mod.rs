//! Interactive remote-access widgets: PTY terminal, VNC and RDP viewers.
//!
//! Rust-only extensions behind opt-in cargo features (`term`, `term-ssh`,
//! `vnc`, `rdp`, or all via `widgets`) — NOT part of the frozen v1.0 API
//! contract. Each widget is a dedicated per-connection WebSocket under the
//! protected router group, enabled at runtime by a `with_*()` builder or env
//! flag. The session engines live in [`forge_core::widgets`]; this module is
//! the WebSocket route layer, adapting each connection to a
//! [`WidgetStream`]. `/api/term` hands authenticated users a real shell (RCE
//! by design); VNC/RDP open outbound connections. Trusted dev contexts only —
//! see docs/widgets-protocol.md.

#[cfg(feature = "rdp")]
pub mod rdp;
#[cfg(feature = "term")]
pub mod term;
#[cfg(feature = "vnc")]
pub mod vnc;

#[cfg(any(feature = "vnc", feature = "rdp"))]
pub use forge_core::widgets::keymap;
#[cfg(any(feature = "vnc", feature = "rdp"))]
pub use forge_core::widgets::DesktopConfig;
#[cfg(feature = "term")]
pub use forge_core::widgets::TermConfig;
pub use forge_core::widgets::{proto, StreamClosed, WidgetMsg, WidgetStream, CHANNEL_CAP};

use axum::extract::ws::{Message, WebSocket};

/// [`WidgetStream`] over an axum WebSocket: text = control JSON, binary =
/// payload. axum answers protocol-level pings itself, so they never surface.
pub struct WsStream(pub WebSocket);

impl WidgetStream for WsStream {
    async fn recv(&mut self) -> Option<WidgetMsg> {
        loop {
            match self.0.recv().await {
                Some(Ok(Message::Text(text))) => return Some(WidgetMsg::Text(text.to_string())),
                Some(Ok(Message::Binary(bytes))) => return Some(WidgetMsg::Binary(bytes.to_vec())),
                Some(Ok(Message::Close(_))) => return Some(WidgetMsg::Close),
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return None,
            }
        }
    }

    async fn send(&mut self, msg: WidgetMsg) -> Result<(), StreamClosed> {
        let msg = match msg {
            WidgetMsg::Text(text) => Message::Text(text.into()),
            WidgetMsg::Binary(bytes) => Message::Binary(bytes.into()),
            WidgetMsg::Close => Message::Close(None),
        };
        self.0.send(msg).await.map_err(|_| StreamClosed)
    }
}
