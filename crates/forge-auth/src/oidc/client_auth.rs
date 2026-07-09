//! Token-endpoint client authentication: `client_secret_basic`,
//! `client_secret_post`, or `none` (public clients — PKCE enforced by the
//! grant handlers).

use axum::http::HeaderMap;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::db::models::Client;
use crate::error::OAuthError;
use crate::state::SharedState;
use crate::util::verify_password;

fn urldecode(s: &str) -> String {
    url::form_urlencoded::parse(format!("v={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| s.to_string())
}

fn basic_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let raw = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = raw.strip_prefix("Basic ")?;
    let decoded = B64.decode(encoded).ok()?;
    let text = String::from_utf8(decoded).ok()?;
    let (id, secret) = text.split_once(':')?;
    Some((urldecode(id), urldecode(secret)))
}

/// Authenticate the client for a token-endpoint request. `form_id`/`form_secret`
/// come from the request body (`client_secret_post` / public clients).
pub async fn authenticate(
    state: &SharedState,
    headers: &HeaderMap,
    form_id: Option<&str>,
    form_secret: Option<&str>,
) -> Result<Client, OAuthError> {
    let (client_id, secret) = match basic_credentials(headers) {
        Some((id, secret)) => (id, Some(secret)),
        None => match form_id {
            Some(id) => (id.to_string(), form_secret.map(String::from)),
            None => return Err(OAuthError::invalid_client("missing client credentials")),
        },
    };

    let client = state
        .db
        .client_by_id(&client_id)
        .await?
        .filter(|c| !c.disabled)
        .ok_or_else(|| OAuthError::invalid_client("unknown client"))?;

    match client.client_type.as_str() {
        "public" => Ok(client),
        _ => {
            let secret = secret.filter(|s| !s.is_empty()).ok_or_else(|| {
                OAuthError::invalid_client("confidential client requires a secret")
            })?;
            let hash = client
                .secret_hash
                .as_deref()
                .ok_or_else(|| OAuthError::invalid_client("client has no secret configured"))?;
            if !verify_password(&secret, hash) {
                return Err(OAuthError::invalid_client("bad client secret"));
            }
            Ok(client)
        }
    }
}
