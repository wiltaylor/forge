//! GET /api/term — PTY terminal WebSocket route.
//!
//! Session logic lives in [`forge_core::widgets::term`]; this handler only
//! authenticates, pulls the runtime config and hands the upgraded socket to
//! the engine.

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::Response;

use super::WsStream;
use crate::auth::extract::RequireClaims;
use crate::state::ForgeState;

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ForgeState>,
    _claims: RequireClaims,
) -> Response {
    let config = state
        .inner
        .term
        .clone()
        .expect("route mounted without term config");
    ws.on_upgrade(move |socket| forge_core::widgets::term::session(WsStream(socket), config))
}
