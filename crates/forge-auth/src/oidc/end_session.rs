//! `GET/POST /oauth2/logout` — OIDC RP-initiated logout.

use axum::extract::Query;
use axum::response::{IntoResponse, Redirect};
use axum::Extension;
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::error::AppError;
use crate::session::{clear_cookie, COOKIE_NAME};
use crate::state::SharedState;
use crate::util::sha256_hex;

#[derive(Deserialize)]
pub struct EndSessionParams {
    #[serde(default)]
    pub id_token_hint: Option<String>,
    #[serde(default)]
    pub post_logout_redirect_uri: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
}

pub async fn end_session(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    Query(params): Query<EndSessionParams>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(cookie) = jar.get(COOKIE_NAME) {
        state.db.session_revoke(&sha256_hex(cookie.value())).await?;
    }
    let jar = CookieJar::new().add(clear_cookie(&state));

    // Only redirect to a URI registered on the (hinted) client.
    let target = match params.post_logout_redirect_uri.as_deref() {
        Some(uri) => {
            let client_id = params
                .client_id
                .clone()
                .or_else(|| client_id_from_hint(&state, params.id_token_hint.as_deref()));
            let registered = match client_id {
                Some(cid) => state
                    .db
                    .client_by_id(&cid)
                    .await?
                    .map(|c| c.post_logout_redirect_uris.iter().any(|u| u == uri))
                    .unwrap_or(false),
                None => false,
            };
            if registered {
                let mut url = url::Url::parse(uri).map_err(|_| AppError::BadRequest("bad post_logout_redirect_uri".into()))?;
                if let Some(s) = params.state.as_deref() {
                    url.query_pairs_mut().append_pair("state", s);
                }
                url.to_string()
            } else {
                "/login".to_string()
            }
        }
        None => "/login".to_string(),
    };
    Ok((jar, Redirect::to(&target)))
}

/// Best-effort `aud` extraction from the id_token_hint. The signature doesn't
/// need verifying just to pick the client for redirect validation — the
/// redirect target itself is checked against the registered allowlist.
fn client_id_from_hint(_state: &SharedState, hint: Option<&str>) -> Option<String> {
    let hint = hint?;
    let mut parts = hint.split('.');
    let (_h, payload) = (parts.next()?, parts.next()?);
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    value.get("aud")?.as_str().map(String::from)
}
