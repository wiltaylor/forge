//! GET /api/health — unauthenticated liveness + capability report.

use axum::extract::State;
use axum::response::Response;

use crate::envelope::ok;
use crate::state::ForgeState;

pub(crate) async fn health(State(state): State<ForgeState>) -> Response {
    ok(forge_core::health_payload(
        state.app(),
        state.uptime_s(),
        env!("CARGO_PKG_VERSION"),
        state.auth_enabled(),
        &state.action_names(),
    ))
}
