//! Upstream identity connectors: OIDC (generic + Google/Entra presets),
//! GitHub OAuth2, and LDAP bind.
//!
//! Flow state rides on an `auth_requests` row: when the login started from
//! `/oauth2/authorize` the row already exists (and holds the RP's request);
//! a direct "Sign in with …" click creates a bare row. The row id doubles as
//! the OAuth `state` parameter — it is random and single-use.

pub mod github;
pub mod ldap;
pub mod linking;
pub mod oidc;

use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Extension;
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::db::models::UpstreamProvider;
use crate::error::AppError;
use crate::session::start_session;
use crate::state::SharedState;
use crate::util::new_id;

const UPSTREAM_REQUEST_TTL: i64 = 600;

#[derive(Deserialize)]
pub struct StartParams {
    #[serde(default)]
    pub request: Option<String>,
}

/// `GET /api/login/upstream/{slug}` — 302 to the provider.
pub async fn start_upstream_login(
    Extension(state): Extension<SharedState>,
    Path(slug): Path<String>,
    Query(params): Query<StartParams>,
) -> Result<Response, AppError> {
    let provider = state
        .db
        .provider_by_slug(&slug)
        .await?
        .filter(|p| p.enabled && p.kind != "ldap")
        .ok_or(AppError::NotFound)?;

    // Reuse the pending /authorize request or create a bare one.
    let request_id = match &params.request {
        Some(id) => {
            state
                .db
                .auth_request_get(id)
                .await?
                .ok_or(AppError::NotFound)?;
            id.clone()
        }
        None => {
            let id = new_id();
            state
                .db
                .auth_request_create(&id, None, &serde_json::json!({}), UPSTREAM_REQUEST_TTL)
                .await?;
            id
        }
    };

    let (url, mut upstream) = match provider.kind.as_str() {
        "oidc" => oidc::start(&state, &provider, &request_id).await?,
        "github" => github::start(&state, &provider, &request_id)?,
        other => {
            return Err(AppError::BadRequest(format!(
                "cannot start login for kind {other:?}"
            )))
        }
    };
    upstream["slug"] = serde_json::json!(provider.slug);
    state
        .db
        .auth_request_set_upstream(&request_id, &upstream)
        .await?;
    Ok(Redirect::to(&url).into_response())
}

#[derive(Deserialize)]
pub struct CallbackParams {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
}

fn login_error_redirect(request_id: Option<&str>, error: &str) -> Response {
    let target = match request_id {
        Some(id) => format!("/login?request={id}&error={error}"),
        None => format!("/login?error={error}"),
    };
    Redirect::to(&target).into_response()
}

/// `GET /api/callback/{slug}` — provider redirects back here.
pub async fn upstream_callback(
    Extension(state): Extension<SharedState>,
    Path(slug): Path<String>,
    Query(params): Query<CallbackParams>,
) -> Result<Response, AppError> {
    let Some(request_id) = params.state.as_deref() else {
        return Ok(login_error_redirect(None, "upstream_failed"));
    };
    let Some(request) = state.db.auth_request_get(request_id).await? else {
        return Ok(login_error_redirect(None, "upstream_failed"));
    };
    // A bare row (no client) is only flow state; a real one resumes /authorize.
    let resumes_authorize = request.client_id.is_some();
    let public_request_id = resumes_authorize.then_some(request_id);

    if let Some(err) = &params.error {
        tracing::warn!(slug, error = %err, description = ?params.error_description, "upstream login returned an error");
        return Ok(login_error_redirect(public_request_id, "upstream_failed"));
    }

    let provider = state
        .db
        .provider_by_slug(&slug)
        .await?
        .filter(|p| p.enabled)
        .ok_or(AppError::NotFound)?;
    let upstream = request.upstream.clone().unwrap_or_default();
    if upstream.get("slug").and_then(|v| v.as_str()) != Some(provider.slug.as_str()) {
        return Ok(login_error_redirect(public_request_id, "upstream_failed"));
    }
    let Some(code) = params.code.as_deref() else {
        return Ok(login_error_redirect(public_request_id, "upstream_failed"));
    };

    let profile = match provider.kind.as_str() {
        "oidc" => oidc::finish(&state, &provider, &upstream, code).await,
        "github" => github::finish(&state, &provider, code).await,
        other => Err(AppError::BadRequest(format!("bad provider kind {other:?}"))),
    };
    let profile = match profile {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(slug, error = %e, "upstream login failed");
            return Ok(login_error_redirect(public_request_id, "upstream_failed"));
        }
    };

    let user = match linking::resolve_user(&state, &provider, &profile).await {
        Ok(user) => user,
        Err(AppError::Forbidden) => {
            return Ok(login_error_redirect(public_request_id, "link_denied"));
        }
        Err(e) => return Err(e),
    };

    let amr = vec![format!("federated:{}", provider.slug)];
    let cookie = start_session(&state, &user.id, &amr).await?;
    let jar = CookieJar::new().add(cookie);

    let target = if resumes_authorize {
        format!("/oauth2/authorize?request={request_id}")
    } else {
        state.db.auth_request_delete(request_id).await?;
        "/account".to_string()
    };
    Ok((jar, Redirect::to(&target)).into_response())
}

/// Admin "test connection" hook.
pub async fn test_provider(
    state: &SharedState,
    provider: &UpstreamProvider,
) -> Result<String, AppError> {
    match provider.kind.as_str() {
        "oidc" => oidc::test(state, provider).await,
        "github" => github::test(provider),
        "ldap" => ldap::test(state, provider).await,
        other => Err(AppError::BadRequest(format!(
            "unknown provider kind {other:?}"
        ))),
    }
}
