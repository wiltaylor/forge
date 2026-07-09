use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::db::models::Client;
use crate::error::AppError;
use crate::session::AdminUser;
use crate::state::SharedState;
use crate::util::{hash_password, now, random_token};

pub async fn list(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let clients = state.db.clients_list().await?;
    let out: Vec<_> = clients.iter().map(client_json).collect();
    Ok(ok(out))
}

fn client_json(client: &Client) -> serde_json::Value {
    let mut value = serde_json::to_value(client).expect("client serializes");
    value["has_secret"] = json!(client.secret_hash.is_some());
    value["has_legacy_secret"] = json!(client.has_legacy_secret());
    value
}

#[derive(Deserialize)]
pub struct ClientBody {
    pub id: Option<String>,
    pub name: String,
    #[serde(default = "default_client_type")]
    pub client_type: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub post_logout_redirect_uris: Vec<String>,
    #[serde(default = "default_scopes")]
    pub allowed_scopes: Vec<String>,
    #[serde(default = "default_grants")]
    pub allowed_grants: Vec<String>,
    #[serde(default)]
    pub access_token_ttl: Option<i64>,
    #[serde(default)]
    pub refresh_token_ttl: Option<i64>,
    #[serde(default)]
    pub role_mappings: Option<serde_json::Value>,
    #[serde(default)]
    pub claims_config: Option<serde_json::Value>,
    #[serde(default)]
    pub exchange_audiences: Vec<String>,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub legacy_hs256_secret: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

fn default_client_type() -> String {
    "confidential".into()
}
fn default_scopes() -> Vec<String> {
    vec!["openid".into(), "profile".into(), "email".into(), "roles".into()]
}
fn default_grants() -> Vec<String> {
    vec!["authorization_code".into(), "refresh_token".into()]
}

fn validate_body(body: &ClientBody) -> Result<(), AppError> {
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("client name is required".into()));
    }
    if !matches!(body.client_type.as_str(), "confidential" | "public") {
        return Err(AppError::BadRequest("client_type must be confidential or public".into()));
    }
    for uri in &body.redirect_uris {
        let parsed = url::Url::parse(uri)
            .map_err(|_| AppError::BadRequest(format!("invalid redirect_uri {uri:?}")))?;
        if parsed.fragment().is_some() {
            return Err(AppError::BadRequest("redirect_uri must not contain a fragment".into()));
        }
    }
    if let Some(secret) = &body.legacy_hs256_secret {
        if !secret.is_empty() && secret.len() < 32 {
            return Err(AppError::BadRequest(
                "legacy_hs256_secret must be at least 32 characters (forge-server requirement)".into(),
            ));
        }
    }
    Ok(())
}

fn build_client(id: String, body: &ClientBody) -> Client {
    Client {
        id,
        name: body.name.trim().to_string(),
        client_type: body.client_type.clone(),
        secret_hash: None,
        redirect_uris: body.redirect_uris.clone(),
        post_logout_redirect_uris: body.post_logout_redirect_uris.clone(),
        allowed_scopes: body.allowed_scopes.clone(),
        allowed_grants: body.allowed_grants.clone(),
        access_token_ttl: body.access_token_ttl,
        refresh_token_ttl: body.refresh_token_ttl,
        role_mappings: body.role_mappings.clone().filter(|v| !v.is_null()),
        claims_config: body.claims_config.clone().filter(|v| !v.is_null()),
        exchange_audiences: body.exchange_audiences.clone(),
        trusted: body.trusted,
        legacy_hs256_secret: body.legacy_hs256_secret.clone().filter(|s| !s.is_empty()),
        disabled: body.disabled,
        created_at: now(),
    }
}

pub async fn create(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Json(body): Json<ClientBody>,
) -> Result<impl IntoResponse, AppError> {
    validate_body(&body)?;
    let id = body
        .id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(crate::util::new_id);
    let mut client = build_client(id, &body);

    // Confidential clients get a generated secret, returned exactly once.
    let secret = if client.client_type == "confidential" {
        let secret = random_token("fac_");
        client.secret_hash = Some(hash_password(&secret)?);
        Some(secret)
    } else {
        None
    };
    state.db.client_create(&client).await?;
    let mut value = client_json(&client);
    value["client_secret"] = json!(secret);
    Ok(ok(value))
}

pub async fn get(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let client = state.db.client_by_id(&id).await?.ok_or(AppError::NotFound)?;
    Ok(ok(client_json(&client)))
}

pub async fn update(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<ClientBody>,
) -> Result<impl IntoResponse, AppError> {
    validate_body(&body)?;
    let existing = state.db.client_by_id(&id).await?.ok_or(AppError::NotFound)?;
    let mut client = build_client(id, &body);
    client.secret_hash = existing.secret_hash;
    client.created_at = existing.created_at;
    // Absent field = keep the stored legacy secret; empty string = clear it.
    client.legacy_hs256_secret = match body.legacy_hs256_secret.as_deref() {
        None => existing.legacy_hs256_secret,
        Some("") => None,
        Some(s) => Some(s.to_string()),
    };
    state.db.client_update(&client).await?;
    Ok(ok(client_json(&client)))
}

pub async fn delete(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if !state.db.client_delete(&id).await? {
        return Err(AppError::NotFound);
    }
    Ok(ok(json!({ "deleted": true })))
}

/// Regenerate the client secret; the new value is returned exactly once.
pub async fn regenerate_secret(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let client = state.db.client_by_id(&id).await?.ok_or(AppError::NotFound)?;
    if client.client_type != "confidential" {
        return Err(AppError::BadRequest("public clients have no secret".into()));
    }
    let secret = random_token("fac_");
    state.db.client_set_secret(&id, &hash_password(&secret)?).await?;
    Ok(ok(json!({ "client_id": id, "client_secret": secret })))
}
