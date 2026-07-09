//! GET /api/health — unauthenticated liveness + capability report.

use axum::extract::State;
use axum::response::Response;
use serde_json::json;

use crate::envelope::ok;
use crate::state::ForgeState;

pub(crate) async fn health(State(state): State<ForgeState>) -> Response {
    let uptime = (state.uptime_s() * 10.0).round() / 10.0;
    ok(json!({
        "uptime_s": uptime,
        "version": env!("CARGO_PKG_VERSION"),
        "app": state.app(),
        "auth_enabled": state.auth_enabled(),
        "actions": state.action_names(),
    }))
}
