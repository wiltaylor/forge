//! Browser sessions: opaque HttpOnly cookie, row in `sessions`, extractors
//! for handlers.
//!
//! CSRF: the cookie is SameSite=Lax and every mutating `/api` request must
//! carry `X-Forge-Auth: 1` (enforced by the extractors) — a cross-site form
//! post can neither set that header nor ride the cookie on a subresource.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Method;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

use crate::db::models::{Session, User};
use crate::error::AppError;
use crate::state::SharedState;
use crate::tokens::access::validate_access;
use crate::util::{random_token, sha256_hex};

pub const COOKIE_NAME: &str = "forge_auth_session";
pub const CSRF_HEADER: &str = "x-forge-auth";

/// Create a session row and return the Set-Cookie for it.
pub async fn start_session(
    state: &SharedState,
    user_id: &str,
    amr: &[String],
) -> Result<Cookie<'static>, AppError> {
    let value = random_token("fas_");
    state
        .db
        .session_create(&sha256_hex(&value), user_id, amr, state.cfg.session_idle_ttl)
        .await?;
    Ok(build_cookie(state, value))
}

pub fn clear_cookie(state: &SharedState) -> Cookie<'static> {
    let mut cookie = build_cookie(state, String::new());
    cookie.make_removal();
    cookie
}

fn build_cookie(state: &SharedState, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(COOKIE_NAME, value);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(state.cfg.cookie_secure);
    cookie
}

fn shared_state(parts: &Parts) -> Result<SharedState, AppError> {
    parts
        .extensions
        .get::<SharedState>()
        .cloned()
        .ok_or_else(|| AppError::Internal("AppState extension missing".into()))
}

fn csrf_ok(parts: &Parts) -> bool {
    matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS)
        || parts.headers.contains_key(CSRF_HEADER)
}

async fn session_from_parts(parts: &Parts) -> Result<Option<(SharedState, Session, User)>, AppError> {
    let state = shared_state(parts)?;
    let jar = CookieJar::from_headers(&parts.headers);
    let Some(cookie) = jar.get(COOKIE_NAME) else { return Ok(None) };
    let Some(session) = state
        .db
        .session_touch(
            &sha256_hex(cookie.value()),
            state.cfg.session_idle_ttl,
            state.cfg.session_absolute_ttl,
        )
        .await?
    else {
        return Ok(None);
    };
    let Some(user) = state.db.user_by_id(&session.user_id).await? else { return Ok(None) };
    if user.disabled {
        return Ok(None);
    }
    Ok(Some((state, session, user)))
}

/// A live browser session. Rejects with 401 when absent/expired, 403 when the
/// CSRF header is missing on a mutating request.
pub struct SessionUser {
    pub session: Session,
    pub user: User,
    pub roles: Vec<String>,
}

impl<S: Send + Sync> FromRequestParts<S> for SessionUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _s: &S) -> Result<Self, Self::Rejection> {
        if !csrf_ok(parts) {
            return Err(AppError::Forbidden);
        }
        let Some((state, session, user)) = session_from_parts(parts).await? else {
            return Err(AppError::Unauthorized);
        };
        let roles = state.db.user_role_names(&user.id).await?;
        Ok(Self { session, user, roles })
    }
}

/// Like [`SessionUser`] but never rejects on a missing session.
pub struct MaybeSession(pub Option<SessionUser>);

impl<S: Send + Sync> FromRequestParts<S> for MaybeSession {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, s: &S) -> Result<Self, Self::Rejection> {
        match SessionUser::from_request_parts(parts, s).await {
            Ok(u) => Ok(Self(Some(u))),
            Err(AppError::Unauthorized) => Ok(Self(None)),
            Err(e) => Err(e),
        }
    }
}

/// Admin access: a session (or one of our RS256 Bearer tokens) carrying the
/// `admin` role.
pub struct AdminUser {
    pub user: User,
}

impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, s: &S) -> Result<Self, Self::Rejection> {
        // Bearer token path (programmatic access) takes priority when present.
        let bearer = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if let Some(token) = bearer {
            let state = shared_state(parts)?;
            let keys = state.keys.read().await;
            let claims = validate_access(&keys, &state.cfg.issuer, token)?;
            drop(keys);
            if !claims.roles.iter().any(|r| r == "admin") {
                return Err(AppError::Forbidden);
            }
            let user = state
                .db
                .user_by_id(&claims.sub)
                .await?
                .filter(|u| !u.disabled)
                .ok_or(AppError::Unauthorized)?;
            return Ok(Self { user });
        }

        let session = SessionUser::from_request_parts(parts, s).await?;
        if !session.roles.iter().any(|r| r == "admin") {
            return Err(AppError::Forbidden);
        }
        Ok(Self { user: session.user })
    }
}
