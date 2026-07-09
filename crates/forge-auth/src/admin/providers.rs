use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::error::AppError;
use crate::session::AdminUser;
use crate::state::SharedState;

const KINDS: &[&str] = &["oidc", "github", "ldap"];

fn provider_json(p: &crate::db::models::UpstreamProvider) -> serde_json::Value {
    let mut value = serde_json::to_value(p).expect("provider serializes");
    // Don't echo upstream secrets back to the browser.
    if let Some(config) = value.get_mut("config").and_then(|c| c.as_object_mut()) {
        for key in ["client_secret", "bind_password"] {
            if config.contains_key(key) {
                config.insert(key.into(), json!("__secret__"));
            }
        }
    }
    value
}

pub async fn list(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let providers = state.db.providers_list().await?;
    Ok(ok(providers.iter().map(provider_json).collect::<Vec<_>>()))
}

#[derive(Deserialize)]
pub struct ProviderBody {
    pub slug: String,
    pub kind: String,
    pub display_name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub allow_signup: bool,
    #[serde(default)]
    pub link_by_email: bool,
    #[serde(default)]
    pub config: serde_json::Value,
    /// `[{"external_group": "...", "role": "role-name"}]`
    #[serde(default)]
    pub group_mappings: Option<Vec<GroupMappingBody>>,
}

#[derive(Deserialize)]
pub struct GroupMappingBody {
    pub external_group: String,
    pub role: String,
}

fn default_true() -> bool {
    true
}

pub async fn upsert(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Json(body): Json<ProviderBody>,
) -> Result<impl IntoResponse, AppError> {
    let slug = body.slug.trim().to_lowercase();
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::BadRequest("slug must be alphanumeric with - or _".into()));
    }
    if !KINDS.contains(&body.kind.as_str()) {
        return Err(AppError::BadRequest(format!("kind must be one of {KINDS:?}")));
    }
    // Preserve stored secrets when the UI round-trips the `__secret__` mask.
    let mut config = body.config.clone();
    if let Some(existing) = state.db.provider_by_slug(&slug).await? {
        if let (Some(new_cfg), Some(old_cfg)) = (config.as_object_mut(), existing.config.as_object()) {
            for key in ["client_secret", "bind_password"] {
                if new_cfg.get(key).and_then(|v| v.as_str()) == Some("__secret__") {
                    if let Some(old) = old_cfg.get(key) {
                        new_cfg.insert(key.into(), old.clone());
                    }
                }
            }
        }
    }
    let provider = state
        .db
        .provider_upsert(
            &slug,
            &body.kind,
            body.display_name.trim(),
            body.enabled,
            body.allow_signup,
            body.link_by_email,
            &config,
        )
        .await?;
    if let Some(mappings) = &body.group_mappings {
        let mut resolved = Vec::with_capacity(mappings.len());
        for m in mappings {
            let role = state
                .db
                .role_by_name(&m.role)
                .await?
                .ok_or_else(|| AppError::BadRequest(format!("unknown role {:?}", m.role)))?;
            resolved.push((m.external_group.clone(), role.id));
        }
        state.db.group_mappings_replace(&provider.id, &resolved).await?;
    }
    Ok(ok(provider_json(&provider)))
}

pub async fn get(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let provider = state.db.provider_by_id(&id).await?.ok_or(AppError::NotFound)?;
    let mappings = state.db.group_mappings_for_provider(&id).await?;
    // Round-trip role *names* — that's what the upsert body takes.
    let roles = state.db.roles_list().await?;
    let mapped: Vec<_> = mappings
        .iter()
        .filter_map(|m| {
            roles
                .iter()
                .find(|r| r.id == m.role_id)
                .map(|r| json!({ "external_group": m.external_group, "role": r.name }))
        })
        .collect();
    let mut value = provider_json(&provider);
    value["group_mappings"] = json!(mapped);
    Ok(ok(value))
}

pub async fn delete(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if !state.db.provider_delete(&id).await? {
        return Err(AppError::NotFound);
    }
    Ok(ok(json!({ "deleted": true })))
}

/// Connectivity test (LDAP service bind; OIDC discovery fetch).
pub async fn test(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let provider = state.db.provider_by_id(&id).await?.ok_or(AppError::NotFound)?;
    let result = crate::upstream::test_provider(&state, &provider).await;
    match result {
        Ok(detail) => Ok(ok(json!({ "ok": true, "detail": detail }))),
        Err(e) => Ok(ok(json!({ "ok": false, "detail": e.to_string() }))),
    }
}
