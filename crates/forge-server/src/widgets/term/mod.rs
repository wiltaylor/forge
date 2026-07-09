//! GET /api/term — PTY terminal WebSocket (local shell + SSH).
//!
//! Binary frames carry raw tty bytes both ways; JSON text frames carry
//! control ([`TermClientMsg`] / [`TermServerMsg`]). The first client frame
//! must be `start`, which picks local vs ssh.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;

use super::proto::TermServerMsg;
use super::TermConfig;
use crate::auth::jwt::Claims;
use crate::state::ForgeState;

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ForgeState>,
    _claims: Claims,
) -> Response {
    let config = state
        .inner
        .term
        .clone()
        .expect("route mounted without term config");
    ws.on_upgrade(move |socket| session(socket, config))
}

/// Stub until the PTY backend lands: proves auth + upgrade, then errors out.
async fn session(mut socket: WebSocket, _config: Arc<TermConfig>) {
    let msg = TermServerMsg::Error {
        message: "terminal backend not implemented yet".into(),
    };
    if let Ok(text) = serde_json::to_string(&msg) {
        let _ = socket.send(Message::Text(text.into())).await;
    }
    let _ = socket.send(Message::Close(None)).await;
}
