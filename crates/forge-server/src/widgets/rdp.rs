//! GET /api/desktop/rdp — RDP viewer WebSocket (IronRDP).
//!
//! The server runs the RDP session (TLS + CredSSP/NLA) and streams decoded
//! RGBA rect frames ([`super::proto::encode_rect`]); JSON text frames carry
//! control ([`DesktopClientMsg`] / [`DesktopServerMsg`]).

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;

use super::proto::DesktopServerMsg;
use super::DesktopConfig;
use crate::auth::jwt::Claims;
use crate::state::ForgeState;

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ForgeState>,
    _claims: Claims,
) -> Response {
    let config = state
        .inner
        .rdp
        .clone()
        .expect("route mounted without rdp config");
    ws.on_upgrade(move |socket| session(socket, config))
}

/// Stub until the IronRDP backend lands: proves auth + upgrade, then errors out.
async fn session(mut socket: WebSocket, _config: Arc<DesktopConfig>) {
    let msg = DesktopServerMsg::Error {
        message: "rdp backend not implemented yet".into(),
    };
    if let Ok(text) = serde_json::to_string(&msg) {
        let _ = socket.send(Message::Text(text.into())).await;
    }
    let _ = socket.send(Message::Close(None)).await;
}
