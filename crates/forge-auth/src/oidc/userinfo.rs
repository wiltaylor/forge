//! `GET/POST /oauth2/userinfo`.

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

use crate::error::OAuthError;
use crate::state::SharedState;
use crate::tokens::access::validate_access;

pub async fn userinfo(
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, OAuthError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| OAuthError::invalid_client("missing bearer token"))?;

    let keys = state.keys.read().await;
    let claims = validate_access(&keys, &state.cfg.issuer, token)
        .map_err(|_| OAuthError::invalid_client("invalid access token"))?;
    drop(keys);

    let user = state
        .db
        .user_by_id(&claims.sub)
        .await?
        .filter(|u| !u.disabled)
        .ok_or_else(|| OAuthError::invalid_client("user no longer valid"))?;

    let scope = claims.scope.as_str();
    let has = |s: &str| scope.split_whitespace().any(|x| x == s);
    let mut payload = json!({ "sub": user.id, "preferred_username": user.username });
    if has("email") {
        payload["email"] = json!(user.email);
        payload["email_verified"] = json!(user.email_verified);
    }
    if has("profile") {
        payload["name"] = json!(user.display_name);
    }
    if has("roles") || claims.roles.iter().len() > 0 {
        payload["roles"] = json!(claims.roles);
    }
    Ok(Json(payload))
}
