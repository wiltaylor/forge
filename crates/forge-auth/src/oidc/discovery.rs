//! `/.well-known/openid-configuration` and `/.well-known/jwks.json`.

use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

use crate::state::SharedState;

pub async fn openid_configuration(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    let issuer = &state.cfg.issuer;
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/oauth2/authorize"),
        "token_endpoint": format!("{issuer}/oauth2/token"),
        "userinfo_endpoint": format!("{issuer}/oauth2/userinfo"),
        "jwks_uri": format!("{issuer}/.well-known/jwks.json"),
        "end_session_endpoint": format!("{issuer}/oauth2/logout"),
        "revocation_endpoint": format!("{issuer}/oauth2/revoke"),
        "introspection_endpoint": format!("{issuer}/oauth2/introspect"),
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": [
            "authorization_code",
            "refresh_token",
            "client_credentials",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "scopes_supported": super::SUPPORTED_SCOPES,
        "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post", "none"],
        "code_challenge_methods_supported": ["S256"],
        "claims_supported": [
            "iss", "sub", "aud", "exp", "iat", "auth_time", "nonce", "amr",
            "preferred_username", "email", "email_verified", "name", "roles",
        ],
    }))
}

pub async fn jwks(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    let keys = state.keys.read().await;
    Json(keys.jwks.clone())
}
