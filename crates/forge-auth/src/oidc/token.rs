//! `POST /oauth2/token` — grant dispatch: authorization_code (+PKCE),
//! refresh_token (rotation with reuse detection), and RFC 8693 token
//! exchange with per-client claim selection.

use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Form, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use super::client_auth;
use crate::db::models::{Client, User};
use crate::error::OAuthError;
use crate::state::SharedState;
use crate::tokens::access::{
    sign_access, sign_client_access, sign_id_token, sign_legacy_hs256, validate_access, Mint,
};
use crate::util::{new_id, pkce_s256, random_token, sha256_hex};

pub const GRANT_TOKEN_EXCHANGE: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const TOKEN_TYPE_ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

#[derive(Debug, Deserialize)]
pub struct TokenForm {
    pub grant_type: Option<String>,
    // client auth (client_secret_post / public)
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    // authorization_code
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    // refresh_token
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    // token exchange
    pub subject_token: Option<String>,
    pub subject_token_type: Option<String>,
    pub audience: Option<String>,
    pub requested_token_type: Option<String>,
}

pub async fn token(
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Result<impl IntoResponse, OAuthError> {
    let client = client_auth::authenticate(
        &state,
        &headers,
        form.client_id.as_deref(),
        form.client_secret.as_deref(),
    )
    .await?;

    let grant_type = form
        .grant_type
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing grant_type"))?;
    if grant_type != GRANT_TOKEN_EXCHANGE && !client.allowed_grants.iter().any(|g| g == grant_type)
    {
        return Err(OAuthError::invalid_grant(format!(
            "grant {grant_type:?} not allowed for this client"
        )));
    }

    let payload = match grant_type {
        "authorization_code" => authorization_code(&state, &client, &form).await?,
        "refresh_token" => refresh(&state, &client, &form).await?,
        "client_credentials" => client_credentials(&state, &client, &form).await?,
        GRANT_TOKEN_EXCHANGE => {
            if !client.exchange_audiences.is_empty()
                || client
                    .allowed_grants
                    .iter()
                    .any(|g| g == GRANT_TOKEN_EXCHANGE)
            {
                exchange(&state, &client, &form).await?
            } else {
                return Err(OAuthError::invalid_grant(
                    "token exchange not allowed for this client",
                ));
            }
        }
        other => return Err(OAuthError::unsupported_grant_type(format!("{other:?}"))),
    };
    Ok(no_store(Json(payload)))
}

fn no_store(json: Json<Value>) -> impl IntoResponse {
    (
        [
            (axum::http::header::CACHE_CONTROL, "no-store"),
            (axum::http::header::PRAGMA, "no-cache"),
        ],
        json,
    )
}

async fn load_user(state: &SharedState, user_id: &str) -> Result<(User, Vec<String>), OAuthError> {
    let user = state
        .db
        .user_by_id(user_id)
        .await?
        .filter(|u| !u.disabled)
        .ok_or_else(|| OAuthError::invalid_grant("user no longer valid"))?;
    let roles = state.db.user_role_names(&user.id).await?;
    Ok((user, roles))
}

/// Mint the standard success payload for a user+client pair.
#[allow(clippy::too_many_arguments)]
async fn mint_tokens(
    state: &SharedState,
    client: &Client,
    user: &User,
    roles: &[String],
    scope: &str,
    amr: &[String],
    auth_time: i64,
    nonce: Option<&str>,
    issue_refresh: bool,
    refresh_family: Option<(&str, &str)>, // (family_id, parent_id) when rotating
) -> Result<Value, OAuthError> {
    let keys = state.keys.read().await;
    let mint = Mint {
        user,
        client,
        roles,
        scope,
        amr,
        auth_time,
        nonce,
        azp: None,
    };
    let (access_token, expires_in) =
        sign_access(&keys, &state.cfg.issuer, state.cfg.access_ttl, &mint)?;
    let id_token = if scope.split_whitespace().any(|s| s == "openid") {
        Some(sign_id_token(
            &keys,
            &state.cfg.issuer,
            state.cfg.access_ttl,
            &mint,
        )?)
    } else {
        None
    };
    drop(keys);

    let refresh_token = if issue_refresh {
        let value = random_token("rt_");
        let ttl = client.refresh_token_ttl.unwrap_or(state.cfg.refresh_ttl);
        let (family, parent) = match refresh_family {
            Some((family, parent)) => (family.to_string(), Some(parent.to_string())),
            None => (new_id(), None),
        };
        state
            .db
            .refresh_token_insert(
                &sha256_hex(&value),
                &family,
                &user.id,
                &client.id,
                scope,
                parent.as_deref(),
                ttl,
            )
            .await?;
        Some(value)
    } else {
        None
    };

    let mut payload = json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": expires_in,
        "scope": scope,
    });
    if let Some(rt) = refresh_token {
        payload["refresh_token"] = json!(rt);
    }
    if let Some(idt) = id_token {
        payload["id_token"] = json!(idt);
    }
    Ok(payload)
}

async fn authorization_code(
    state: &SharedState,
    client: &Client,
    form: &TokenForm,
) -> Result<Value, OAuthError> {
    let code = form
        .code
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing code"))?;
    let auth_code = state
        .db
        .auth_code_consume(&sha256_hex(code))
        .await?
        .ok_or_else(|| OAuthError::invalid_grant("code is invalid, expired or already used"))?;

    if auth_code.client_id != client.id {
        return Err(OAuthError::invalid_grant(
            "code was issued to another client",
        ));
    }
    if form.redirect_uri.as_deref() != Some(auth_code.redirect_uri.as_str()) {
        return Err(OAuthError::invalid_grant("redirect_uri mismatch"));
    }
    match (&auth_code.code_challenge, form.code_verifier.as_deref()) {
        (Some(challenge), Some(verifier)) => {
            if pkce_s256(verifier) != *challenge {
                return Err(OAuthError::invalid_grant("PKCE verification failed"));
            }
        }
        (Some(_), None) => return Err(OAuthError::invalid_request("missing code_verifier")),
        (None, _) => {
            if client.client_type == "public" {
                return Err(OAuthError::invalid_grant("public client requires PKCE"));
            }
        }
    }

    let (user, roles) = load_user(state, &auth_code.user_id).await?;
    let issue_refresh = client.allowed_grants.iter().any(|g| g == "refresh_token");
    mint_tokens(
        state,
        client,
        &user,
        &roles,
        &auth_code.scope,
        &auth_code.amr,
        auth_code.auth_time,
        auth_code.nonce.as_deref(),
        issue_refresh,
        None,
    )
    .await
}

async fn refresh(
    state: &SharedState,
    client: &Client,
    form: &TokenForm,
) -> Result<Value, OAuthError> {
    let raw = form
        .refresh_token
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing refresh_token"))?;
    let stored = state
        .db
        .refresh_token_by_hash(&sha256_hex(raw))
        .await?
        .ok_or_else(|| OAuthError::invalid_grant("unknown refresh token"))?;

    if stored.client_id != client.id {
        return Err(OAuthError::invalid_grant(
            "refresh token was issued to another client",
        ));
    }
    if stored.revoked_at.is_some() || stored.expires_at <= crate::util::now() {
        return Err(OAuthError::invalid_grant(
            "refresh token revoked or expired",
        ));
    }
    // Rotation with reuse detection: losing the atomic mark-used race means
    // this token value was presented twice — revoke the whole family.
    if !state.db.refresh_token_mark_used(&stored.id).await? {
        state.db.refresh_family_revoke(&stored.family_id).await?;
        tracing::warn!(client = %client.id, family = %stored.family_id, "refresh token reuse detected — family revoked");
        return Err(OAuthError::invalid_grant("refresh token reuse detected"));
    }

    // Optional scope narrowing (never widening).
    let scope = match form.scope.as_deref() {
        Some(requested) => {
            let granted: Vec<&str> = stored.scope.split_whitespace().collect();
            if !requested.split_whitespace().all(|s| granted.contains(&s)) {
                return Err(OAuthError::invalid_scope(
                    "requested scope exceeds the original grant",
                ));
            }
            requested.to_string()
        }
        None => stored.scope.clone(),
    };

    let (user, roles) = load_user(state, &stored.user_id).await?;
    mint_tokens(
        state,
        client,
        &user,
        &roles,
        &scope,
        &[],
        stored.created_at,
        None,
        true,
        Some((&stored.family_id, &stored.id)),
    )
    .await
}

/// RFC 6749 §4.4: machine-to-machine. A confidential client authenticates with
/// its secret and gets a long-lived JWT where it is its own subject, carrying
/// its configured `client_roles`. No user, no id_token, no refresh token.
async fn client_credentials(
    state: &SharedState,
    client: &Client,
    form: &TokenForm,
) -> Result<Value, OAuthError> {
    if client.client_type != "confidential" {
        return Err(OAuthError::invalid_client(
            "client_credentials requires a confidential client",
        ));
    }
    // Scope: requested must be a subset of the client's allowlist; absent means
    // the full allowlist.
    let scope = match form.scope.as_deref() {
        Some(requested) => {
            let unknown: Vec<&str> = requested
                .split_whitespace()
                .filter(|s| !client.allowed_scopes.iter().any(|a| a == s))
                .collect();
            if !unknown.is_empty() {
                return Err(OAuthError::invalid_scope(format!(
                    "scope(s) not allowed for this client: {}",
                    unknown.join(" ")
                )));
            }
            requested.to_string()
        }
        None => client.allowed_scopes.join(" "),
    };

    let ttl = client.access_token_ttl.unwrap_or(state.cfg.machine_ttl);
    let keys = state.keys.read().await;
    let (access_token, expires_in) =
        sign_client_access(&keys, &state.cfg.issuer, ttl, client, &scope)?;
    drop(keys);
    Ok(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": expires_in,
        "scope": scope,
    }))
}

/// RFC 8693: swap one of our access tokens for a token scoped to another
/// client (`audience`), with that client's role mapping and claim selection
/// applied. This is how one login fans out to many forge apps.
async fn exchange(
    state: &SharedState,
    requester: &Client,
    form: &TokenForm,
) -> Result<Value, OAuthError> {
    if requester.client_type != "confidential" {
        return Err(OAuthError::invalid_client(
            "token exchange requires a confidential client",
        ));
    }
    match form.subject_token_type.as_deref() {
        None | Some(TOKEN_TYPE_ACCESS) => {}
        Some(other) => {
            return Err(OAuthError::invalid_request(format!(
                "unsupported subject_token_type {other:?}"
            )))
        }
    }
    let subject_token = form
        .subject_token
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing subject_token"))?;
    let audience = form
        .audience
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_target("missing audience"))?;

    let keys = state.keys.read().await;
    let subject = validate_access(&keys, &state.cfg.issuer, subject_token)
        .map_err(|_| OAuthError::invalid_grant("subject_token is not a valid access token"))?;
    drop(keys);

    let allowed = audience == requester.id
        || requester
            .exchange_audiences
            .iter()
            .any(|a| a == audience || a == "*");
    if !allowed {
        return Err(OAuthError::invalid_target(format!(
            "client {:?} may not exchange tokens for audience {audience:?}",
            requester.id
        )));
    }
    let target = state
        .db
        .client_by_id(audience)
        .await?
        .filter(|c| !c.disabled)
        .ok_or_else(|| OAuthError::invalid_target("unknown audience"))?;

    let (user, roles) = load_user(state, &subject.sub).await?;

    // Scope: requested ∩ subject's grant, then filtered to the target's allowlist.
    let subject_scopes: Vec<&str> = subject.scope.split_whitespace().collect();
    let base: Vec<&str> = match form.scope.as_deref() {
        Some(requested) => requested
            .split_whitespace()
            .filter(|s| subject_scopes.contains(s))
            .collect(),
        None => subject_scopes.clone(),
    };
    let scope: String = base
        .into_iter()
        .filter(|s| target.allowed_scopes.iter().any(|a| a == s))
        .collect::<Vec<_>>()
        .join(" ");

    // Legacy forge apps get an HS256 token in forge-server's claim shape.
    if let Some(secret) = &target.legacy_hs256_secret {
        let mapped = crate::tokens::access::map_roles(&target, &roles);
        let ttl = target.access_token_ttl.unwrap_or(state.cfg.access_ttl);
        let token = sign_legacy_hs256(secret, &state.cfg.issuer, &user.username, &mapped, ttl)?;
        return Ok(json!({
            "access_token": token,
            "issued_token_type": TOKEN_TYPE_ACCESS,
            "token_type": "Bearer",
            "expires_in": ttl,
            "scope": scope,
        }));
    }

    let keys = state.keys.read().await;
    let mint = Mint {
        user: &user,
        client: &target,
        roles: &roles,
        scope: &scope,
        amr: &subject.amr,
        auth_time: subject.iat,
        nonce: None,
        azp: Some(&requester.id),
    };
    let (access_token, expires_in) =
        sign_access(&keys, &state.cfg.issuer, state.cfg.access_ttl, &mint)?;
    Ok(json!({
        "access_token": access_token,
        "issued_token_type": TOKEN_TYPE_ACCESS,
        "token_type": "Bearer",
        "expires_in": expires_in,
        "scope": scope,
    }))
}
