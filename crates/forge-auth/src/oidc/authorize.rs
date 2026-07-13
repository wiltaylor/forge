//! `GET /oauth2/authorize` — authorization code + PKCE entry point, plus the
//! consent API the SPA calls.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::ok;
use crate::db::models::{AuthCode, Client};
use crate::error::AppError;
use crate::session::{MaybeSession, SessionUser};
use crate::state::SharedState;
use crate::util::{new_id, now, random_token, sha256_hex};

/// How long a pending /authorize request may sit on the login/consent pages.
const AUTH_REQUEST_TTL: i64 = 600;
/// Authorization code lifetime (RFC 6749 recommends ≤ 10 min; we use 60 s).
const CODE_TTL: i64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthorizeParams {
    #[serde(default)]
    pub response_type: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub redirect_uri: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub code_challenge: Option<String>,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    /// Resume marker for a persisted auth request (ours, not spec).
    #[serde(default)]
    pub request: Option<String>,
}

fn bad_request(msg: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Html(format!(
            "<!doctype html><title>forge-auth</title><h1>Authorization error</h1><p>{}</p>",
            html_escape(msg)
        )),
    )
        .into_response()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Error redirect back to the RP (only used once redirect_uri is validated).
fn error_redirect(redirect_uri: &str, error: &str, state: Option<&str>) -> Response {
    let mut url = url::Url::parse(redirect_uri).expect("validated redirect_uri");
    url.query_pairs_mut().append_pair("error", error);
    if let Some(s) = state {
        url.query_pairs_mut().append_pair("state", s);
    }
    Redirect::to(url.as_str()).into_response()
}

pub async fn authorize(
    Extension(state): Extension<SharedState>,
    MaybeSession(session): MaybeSession,
    Query(query): Query<AuthorizeParams>,
) -> Result<Response, AppError> {
    // Resume a persisted request or persist this one.
    let (request_id, params, consented) = match &query.request {
        Some(id) => match state.db.auth_request_get(id).await? {
            Some(req) => {
                let params: AuthorizeParams =
                    serde_json::from_value(req.params.clone()).unwrap_or_default();
                (req.id, params, req.consented)
            }
            None => {
                return Ok(bad_request(
                    "login request expired — start again from the application",
                ))
            }
        },
        None => {
            let id = new_id();
            let value = serde_json::to_value(&query).map_err(AppError::internal)?;
            state
                .db
                .auth_request_create(&id, query.client_id.as_deref(), &value, AUTH_REQUEST_TTL)
                .await?;
            (id, query, false)
        }
    };

    // Hard failures (unknown client / bad redirect_uri) never redirect.
    let Some(client_id) = params.client_id.as_deref() else {
        return Ok(bad_request("missing client_id"));
    };
    let Some(client) = state
        .db
        .client_by_id(client_id)
        .await?
        .filter(|c| !c.disabled)
    else {
        return Ok(bad_request("unknown or disabled client"));
    };
    let Some(redirect_uri) = params.redirect_uri.as_deref() else {
        return Ok(bad_request("missing redirect_uri"));
    };
    if !client.redirect_uris.iter().any(|u| u == redirect_uri) {
        return Ok(bad_request(
            "redirect_uri is not registered for this client",
        ));
    }

    // From here on, errors go back to the RP.
    let rp_state = params.state.as_deref();
    if params.response_type.as_deref() != Some("code") {
        return Ok(error_redirect(
            redirect_uri,
            "unsupported_response_type",
            rp_state,
        ));
    }
    let scope = params.scope.clone().unwrap_or_else(|| "openid".to_string());
    if !super::scope_is_supported(&scope)
        || !super::scope_allowed_for_client(&scope, &client.allowed_scopes)
    {
        return Ok(error_redirect(redirect_uri, "invalid_scope", rp_state));
    }
    if !client
        .allowed_grants
        .iter()
        .any(|g| g == "authorization_code")
    {
        return Ok(error_redirect(
            redirect_uri,
            "unauthorized_client",
            rp_state,
        ));
    }
    match (
        params.code_challenge.as_deref(),
        params.code_challenge_method.as_deref(),
    ) {
        (Some(_), Some("S256")) | (Some(_), None) => {}
        (Some(_), Some(_)) => return Ok(error_redirect(redirect_uri, "invalid_request", rp_state)),
        (None, _) => {
            if client.client_type == "public" {
                // PKCE is mandatory for public clients.
                return Ok(error_redirect(redirect_uri, "invalid_request", rp_state));
            }
        }
    }

    // Need a signed-in user.
    let Some(session) = session else {
        return Ok(Redirect::to(&format!("/login?request={request_id}")).into_response());
    };
    // Consent (trusted first-party clients skip it).
    if !client.trusted && !consented {
        return Ok(Redirect::to(&format!("/consent?request={request_id}")).into_response());
    }

    // Issue the code.
    let code = random_token("fc_");
    let ts = now();
    state
        .db
        .auth_code_insert(&AuthCode {
            code_hash: sha256_hex(&code),
            client_id: client.id.clone(),
            user_id: session.user.id.clone(),
            redirect_uri: redirect_uri.to_string(),
            scope,
            nonce: params.nonce.clone(),
            code_challenge: params.code_challenge.clone(),
            code_challenge_method: Some(
                params
                    .code_challenge_method
                    .clone()
                    .unwrap_or_else(|| "S256".into()),
            )
            .filter(|_| params.code_challenge.is_some()),
            auth_time: session.session.auth_time,
            amr: session.session.amr.clone(),
            created_at: ts,
            expires_at: ts + CODE_TTL,
            consumed_at: None,
        })
        .await?;
    state.db.auth_request_delete(&request_id).await?;

    let mut url = url::Url::parse(redirect_uri).expect("validated redirect_uri");
    url.query_pairs_mut().append_pair("code", &code);
    if let Some(s) = rp_state {
        url.query_pairs_mut().append_pair("state", s);
    }
    Ok(Redirect::to(url.as_str()).into_response())
}

// --- consent API (SPA-facing, forge envelope) ---

pub async fn consent_info(
    Extension(state): Extension<SharedState>,
    _user: SessionUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let request = state
        .db
        .auth_request_get(&id)
        .await?
        .ok_or(AppError::NotFound)?;
    let params: AuthorizeParams = serde_json::from_value(request.params).unwrap_or_default();
    let client: Option<Client> = match params.client_id.as_deref() {
        Some(cid) => state.db.client_by_id(cid).await?,
        None => None,
    };
    let client = client.ok_or(AppError::NotFound)?;
    Ok(ok(json!({
        "client_name": client.name,
        "scopes": params
            .scope
            .as_deref()
            .unwrap_or("openid")
            .split_whitespace()
            .collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
pub struct ConsentBody {
    pub approve: bool,
}

pub async fn consent_decide(
    Extension(state): Extension<SharedState>,
    _user: SessionUser,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<ConsentBody>,
) -> Result<impl IntoResponse, AppError> {
    let request = state
        .db
        .auth_request_get(&id)
        .await?
        .ok_or(AppError::NotFound)?;
    let params: AuthorizeParams = serde_json::from_value(request.params).unwrap_or_default();
    if body.approve {
        state.db.auth_request_set_consented(&id).await?;
        return Ok(ok(
            json!({ "redirect_to": format!("/oauth2/authorize?request={id}") }),
        ));
    }
    // Denied: bounce back to the RP with access_denied when we can.
    state.db.auth_request_delete(&id).await?;
    let redirect_to = match (params.redirect_uri.as_deref(), params.client_id.as_deref()) {
        (Some(uri), Some(cid)) => {
            let registered = state
                .db
                .client_by_id(cid)
                .await?
                .map(|c| c.redirect_uris.iter().any(|u| u == uri))
                .unwrap_or(false);
            if registered {
                let mut url = url::Url::parse(uri).map_err(|_| AppError::NotFound)?;
                url.query_pairs_mut().append_pair("error", "access_denied");
                if let Some(s) = params.state.as_deref() {
                    url.query_pairs_mut().append_pair("state", s);
                }
                url.to_string()
            } else {
                "/account".to_string()
            }
        }
        _ => "/account".to_string(),
    };
    Ok(ok(json!({ "redirect_to": redirect_to })))
}
