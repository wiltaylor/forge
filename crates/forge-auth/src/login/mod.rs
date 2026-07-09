//! Browser-facing login/session endpoints (forge envelope responses).

#[cfg(feature = "dev-login")]
pub mod dev;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::error::AppError;
use crate::session::{clear_cookie, start_session, MaybeSession, SessionUser};
use crate::state::SharedState;
use crate::util::sha256_hex;

pub fn dev_login_enabled() -> bool {
    cfg!(feature = "dev-login")
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Where to send the browser after a successful login. The server is the sole
/// authority: either back into a pending `/authorize` request or to the
/// account page — never a caller-supplied URL.
pub async fn post_login_redirect(
    state: &SharedState,
    request_id: Option<&str>,
) -> Result<String, AppError> {
    if let Some(id) = request_id {
        if state.db.auth_request_get(id).await?.is_some() {
            return Ok(format!("/oauth2/authorize?request={id}"));
        }
    }
    Ok("/account".to_string())
}

pub async fn login(
    Extension(state): Extension<SharedState>,
    Json(body): Json<LoginBody>,
) -> Result<impl IntoResponse, AppError> {
    let limiter_key = body.username.trim().to_lowercase();
    if state.login_limiter.is_blocked(&limiter_key) {
        return Err(AppError::BadRequest(
            "too many failed attempts — try again in a few minutes".into(),
        ));
    }

    let user = state
        .db
        .user_by_username(body.username.trim())
        .await?
        .filter(|u| !u.disabled);

    let verified = match &user {
        Some(user) => match state.db.password_hash_for(&user.id).await? {
            Some(hash) => crate::util::verify_password(&body.password, &hash),
            None => false,
        },
        None => {
            // Constant-shape work for unknown users, then the LDAP fallback
            // may still claim the login.
            let _ = crate::util::verify_password(&body.password, &DUMMY_HASH);
            false
        }
    };

    let (user, amr) = if verified {
        (user.expect("verified implies user"), vec!["pwd".to_string()])
    } else {
        match crate::upstream::ldap::try_ldap_login(&state, body.username.trim(), &body.password).await? {
            Some(user) => (user, vec!["ldap".to_string()]),
            None => {
                state.login_limiter.record_failure(&limiter_key);
                return Err(AppError::Unauthorized);
            }
        }
    };
    state.login_limiter.record_success(&limiter_key);

    let cookie = start_session(&state, &user.id, &amr).await?;
    let redirect_to = post_login_redirect(&state, body.request_id.as_deref()).await?;
    let jar = CookieJar::new().add(cookie);
    Ok((
        jar,
        ok(json!({
            "redirect_to": redirect_to,
            "user": { "id": user.id, "username": user.username },
        })),
    ))
}

// A real argon2id hash (computed once), so unknown-user logins cost the same
// as wrong-password logins.
static DUMMY_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    crate::util::hash_password("forge-auth-dummy-password").expect("argon2 hash")
});

pub async fn logout(
    Extension(state): Extension<SharedState>,
    jar: CookieJar,
    _user: SessionUser,
) -> Result<impl IntoResponse, AppError> {
    if let Some(cookie) = jar.get(crate::session::COOKIE_NAME) {
        state.db.session_revoke(&sha256_hex(cookie.value())).await?;
    }
    let jar = CookieJar::new().add(clear_cookie(&state));
    Ok((jar, ok(json!({ "logged_out": true }))))
}

/// Session info for the SPA (also its "who am I" call).
pub async fn session_info(
    MaybeSession(session): MaybeSession,
) -> Result<impl IntoResponse, AppError> {
    let payload = match session {
        Some(s) => json!({
            "authenticated": true,
            "dev_login": dev_login_enabled(),
            "user": {
                "id": s.user.id,
                "username": s.user.username,
                "email": s.user.email,
                "display_name": s.user.display_name,
                "roles": s.roles,
                "amr": s.session.amr,
            },
        }),
        None => json!({ "authenticated": false, "dev_login": dev_login_enabled() }),
    };
    Ok(ok(payload))
}

/// Context for the hosted login page: which client asked for this login and
/// which upstream providers are available.
pub async fn login_request_info(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let request = state.db.auth_request_get(&id).await?.ok_or(AppError::NotFound)?;
    let client_name = match &request.client_id {
        Some(client_id) => state.db.client_by_id(client_id).await?.map(|c| c.name),
        None => None,
    };
    let providers: Vec<_> = state
        .db
        .providers_enabled()
        .await?
        .into_iter()
        .filter(|p| p.kind != "ldap") // LDAP uses the password form, not a button
        .map(|p| json!({ "slug": p.slug, "display_name": p.display_name, "kind": p.kind }))
        .collect();
    Ok(ok(json!({
        "client_name": client_name,
        "scopes": request.params.get("scope").and_then(|s| s.as_str()).unwrap_or(""),
        "providers": providers,
        "dev_login": dev_login_enabled(),
    })))
}

/// Providers list for a plain `/login` visit (no pending auth request).
pub async fn login_providers(
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let providers: Vec<_> = state
        .db
        .providers_enabled()
        .await?
        .into_iter()
        .filter(|p| p.kind != "ldap")
        .map(|p| json!({ "slug": p.slug, "display_name": p.display_name, "kind": p.kind }))
        .collect();
    Ok(ok(json!({ "providers": providers, "dev_login": dev_login_enabled() })))
}
