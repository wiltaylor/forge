//! POST /api/auth/login and GET /api/auth/me.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;

use crate::auth::jwt::{encode_token, Claims};
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
    // Contract: 404 when auth is disabled. External-issuer mode (validator
    // without a login config) also has no login endpoint.
    let Some(config) = state.auth().and_then(|a| a.config.as_ref()) else {
        return err(StatusCode::NOT_FOUND, "auth is disabled");
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

    let user = config
        .users
        .iter()
        .find(|u| u.name == body.username)
        .filter(|u| u.verify(&body.password));
    let Some(user) = user else {
        return err(StatusCode::UNAUTHORIZED, "invalid username or password");
    };

    let claims = Claims::new(
        &user.name,
        user.roles.clone(),
        config.ttl_secs,
        Some(config.iss.clone()),
    );
    match encode_token(&claims, &config.secret) {
        Ok(token) => ok(json!({
            "token": token,
            "expires_at": claims.exp,
            "user": { "name": user.name, "roles": user.roles },
        })),
        Err(e) => crate::error::error_response(e),
    }
}

async fn me(
    crate::auth::extract::RequireClaims(claims): crate::auth::extract::RequireClaims,
) -> Response {
    ok(claims)
}
