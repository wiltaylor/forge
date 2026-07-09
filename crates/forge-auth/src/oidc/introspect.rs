//! `POST /oauth2/introspect` (RFC 7662).

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Form, Json};
use serde::Deserialize;
use serde_json::json;

use super::client_auth;
use crate::error::OAuthError;
use crate::state::SharedState;
use crate::tokens::access::validate_access;
use crate::util::{now, sha256_hex};

#[derive(Deserialize)]
pub struct IntrospectForm {
    pub token: Option<String>,
    #[allow(dead_code)]
    pub token_type_hint: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

pub async fn introspect(
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
    Form(form): Form<IntrospectForm>,
) -> Result<impl IntoResponse, OAuthError> {
    client_auth::authenticate(
        &state,
        &headers,
        form.client_id.as_deref(),
        form.client_secret.as_deref(),
    )
    .await?;

    let inactive = Json(json!({ "active": false }));
    let Some(token) = form.token.as_deref() else { return Ok(inactive) };

    // Access JWT?
    let keys = state.keys.read().await;
    if let Ok(claims) = validate_access(&keys, &state.cfg.issuer, token) {
        drop(keys);
        return Ok(Json(json!({
            "active": true,
            "token_type": "Bearer",
            "iss": claims.iss,
            "sub": claims.sub,
            "aud": claims.aud,
            "azp": claims.azp,
            "scope": claims.scope,
            "exp": claims.exp,
            "iat": claims.iat,
            "username": claims.preferred_username,
            "roles": claims.roles,
        })));
    }
    drop(keys);

    // Refresh token?
    if let Some(stored) = state.db.refresh_token_by_hash(&sha256_hex(token)).await? {
        let active = stored.revoked_at.is_none() && stored.used_at.is_none() && stored.expires_at > now();
        if active {
            return Ok(Json(json!({
                "active": true,
                "token_type": "refresh_token",
                "sub": stored.user_id,
                "client_id": stored.client_id,
                "scope": stored.scope,
                "exp": stored.expires_at,
                "iat": stored.created_at,
            })));
        }
    }
    Ok(inactive)
}
