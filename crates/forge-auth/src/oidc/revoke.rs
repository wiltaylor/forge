//! `POST /oauth2/revoke` (RFC 7009). Revokes refresh-token families; access
//! JWTs are short-lived and just expire.

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Form};
use serde::Deserialize;

use super::client_auth;
use crate::error::OAuthError;
use crate::state::SharedState;
use crate::util::sha256_hex;

#[derive(Deserialize)]
pub struct RevokeForm {
    pub token: Option<String>,
    #[allow(dead_code)]
    pub token_type_hint: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

pub async fn revoke(
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
    Form(form): Form<RevokeForm>,
) -> Result<impl IntoResponse, OAuthError> {
    let client = client_auth::authenticate(
        &state,
        &headers,
        form.client_id.as_deref(),
        form.client_secret.as_deref(),
    )
    .await?;

    // Per RFC 7009 an unknown token is still a 200.
    if let Some(token) = form.token.as_deref() {
        if let Some(stored) = state.db.refresh_token_by_hash(&sha256_hex(token)).await? {
            if stored.client_id == client.id {
                state.db.refresh_family_revoke(&stored.family_id).await?;
            }
        }
    }
    Ok(axum::http::StatusCode::OK)
}
