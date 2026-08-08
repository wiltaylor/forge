//! POST /api/auth/login and GET /api/auth/me.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use crate::auth::AUTH_DISABLED;
use crate::envelope::{err, ok};
use crate::state::ForgeState;

/// Open routes (no token required): login.
pub(crate) fn open_routes() -> Router<ForgeState> {
    Router::new().route("/api/auth/login", post(login))
}

/// Protected routes: me.
pub(crate) fn protected_routes() -> Router<ForgeState> {
    Router::new().route("/api/auth/me", get(me))
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn login(State(state): State<ForgeState>, body: Bytes) -> Response {
    // Contract: 404 when auth is disabled. External-issuer mode (a validator
    // without a login config) has no login endpoint either. Both are decided
    // before the body is read: with no endpoint here, the body is nobody's
    // business.
    let Some(auth) = state.auth().filter(|a| a.can_login()) else {
        return err(StatusCode::NOT_FOUND, AUTH_DISABLED);
    };

    let body: LoginBody = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(e) => {
            return err(
                StatusCode::BAD_REQUEST,
                format!("body must be JSON {{username, password}}: {e}"),
            )
        }
    };

    match auth.login(&body.username, &body.password) {
        Ok(response) => ok(response),
        Err(e) => crate::error::error_response(e),
    }
}

async fn me(
    crate::auth::extract::RequireClaims(claims): crate::auth::extract::RequireClaims,
) -> Response {
    ok(claims)
}
