//! Claims extraction: `Authorization: Bearer` header first, then a
//! `?token=` query parameter (header wins). When auth is disabled the
//! extractor yields [`Claims::anonymous`].

use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::auth::jwt::Claims;
use crate::envelope;
use crate::error::ForgeError;
use crate::state::ForgeState;

/// Pull the token from the Authorization header or `?token=` query param.
pub(crate) fn token_from(headers: &axum::http::HeaderMap, uri: &Uri) -> Option<String> {
    if let Some(value) = headers.get(AUTHORIZATION) {
        if let Ok(value) = value.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
        }
    }
    let query = uri.query()?;
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == "token" && !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn authenticate(state: &ForgeState, headers: &axum::http::HeaderMap, uri: &Uri) -> Result<Claims, ForgeError> {
    let Some(auth) = state.auth() else {
        // Auth-disabled mode is first-class: everything is open, handlers
        // see an anonymous identity.
        return Ok(Claims::anonymous());
    };
    let Some(token) = token_from(headers, uri) else {
        return Err(ForgeError::Unauthorized(
            "missing token (Authorization: Bearer or ?token=)".into(),
        ));
    };
    auth.validator.validate(&token)
}

/// Extractor for the authenticated identity. Rejects with a 401 envelope
/// when auth is enabled and no valid token accompanies the request.
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    ForgeState: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // The auth route-layer middleware stashes claims in extensions.
        if let Some(claims) = parts.extensions.get::<Claims>() {
            return Ok(claims.clone());
        }
        let state = ForgeState::from_ref(state);
        authenticate(&state, &parts.headers, &parts.uri).map_err(IntoResponse::into_response)
    }
}

/// Like [`Claims`] but never rejects: `Some(claims)` when a valid token is
/// present (or auth is disabled → anonymous), `None` otherwise.
#[derive(Debug, Clone)]
pub struct OptionalClaims(pub Option<Claims>);

impl<S> FromRequestParts<S> for OptionalClaims
where
    S: Send + Sync,
    ForgeState: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Some(claims) = parts.extensions.get::<Claims>() {
            return Ok(OptionalClaims(Some(claims.clone())));
        }
        let state = ForgeState::from_ref(state);
        Ok(OptionalClaims(
            authenticate(&state, &parts.headers, &parts.uri).ok(),
        ))
    }
}

/// Route-layer middleware for protected routers: validates the token (when
/// auth is enabled) and stashes [`Claims`] in request extensions; otherwise
/// stashes [`Claims::anonymous`]. Failures short-circuit with a 401 envelope.
pub(crate) async fn auth_middleware(
    State(state): State<ForgeState>,
    mut req: Request,
    next: Next,
) -> Response {
    match authenticate(&state, req.headers(), req.uri()) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(e) => envelope::err(StatusCode::UNAUTHORIZED, e.to_string()),
    }
}
