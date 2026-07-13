//! Self-service account endpoints for the signed-in user.

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::error::AppError;
use crate::session::SessionUser;
use crate::state::SharedState;
use crate::util::{hash_password, verify_password};

pub async fn profile(
    Extension(state): Extension<SharedState>,
    user: SessionUser,
) -> Result<impl IntoResponse, AppError> {
    let identities = state.db.identities_for_user(&user.user.id).await?;
    let mut identity_infos = Vec::with_capacity(identities.len());
    for identity in &identities {
        let provider = state.db.provider_by_id(&identity.provider_id).await?;
        identity_infos.push(json!({
            "id": identity.id,
            "provider": provider.as_ref().map(|p| p.display_name.clone()),
            "provider_slug": provider.map(|p| p.slug),
            "subject": identity.subject,
            "email": identity.email,
            "created_at": identity.created_at,
        }));
    }
    Ok(ok(json!({
        "user": {
            "id": user.user.id,
            "username": user.user.username,
            "email": user.user.email,
            "display_name": user.user.display_name,
            "roles": user.roles,
        },
        "identities": identity_infos,
        "has_password": state.db.password_hash_for(&user.user.id).await?.is_some(),
    })))
}

#[derive(Deserialize)]
pub struct ChangePassword {
    #[serde(default)]
    pub current_password: Option<String>,
    pub new_password: String,
}

pub async fn change_password(
    Extension(state): Extension<SharedState>,
    user: SessionUser,
    Json(body): Json<ChangePassword>,
) -> Result<impl IntoResponse, AppError> {
    if body.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    // Users with an existing password must prove it; federated-only accounts
    // may set their first password freely.
    if let Some(hash) = state.db.password_hash_for(&user.user.id).await? {
        let current = body.current_password.as_deref().unwrap_or("");
        if !verify_password(current, &hash) {
            return Err(AppError::BadRequest("current password is incorrect".into()));
        }
    }
    state
        .db
        .password_set(&user.user.id, &hash_password(&body.new_password)?)
        .await?;
    Ok(ok(json!({ "updated": true })))
}

pub async fn sessions(
    Extension(state): Extension<SharedState>,
    user: SessionUser,
) -> Result<impl IntoResponse, AppError> {
    let sessions = state.db.sessions_for_user(&user.user.id).await?;
    let current = user.session.id_hash.clone();
    let out: Vec<_> = sessions
        .iter()
        .map(|s| {
            json!({
                "id": s.id_hash,
                "current": s.id_hash == current,
                "amr": s.amr,
                "created_at": s.created_at,
                "last_seen": s.last_seen,
            })
        })
        .collect();
    Ok(ok(out))
}

pub async fn revoke_session(
    Extension(state): Extension<SharedState>,
    user: SessionUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // Only the user's own sessions.
    let owned = state
        .db
        .sessions_for_user(&user.user.id)
        .await?
        .into_iter()
        .any(|s| s.id_hash == id);
    if !owned {
        return Err(AppError::NotFound);
    }
    state.db.session_revoke(&id).await?;
    Ok(ok(json!({ "revoked": true })))
}

pub async fn unlink_identity(
    Extension(state): Extension<SharedState>,
    user: SessionUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // Don't let the user lock themselves out: keep at least one login method.
    let has_password = state.db.password_hash_for(&user.user.id).await?.is_some();
    let identities = state.db.identities_for_user(&user.user.id).await?;
    if !has_password && identities.len() <= 1 {
        return Err(AppError::BadRequest(
            "set a password before unlinking your last identity".into(),
        ));
    }
    if !state.db.identity_unlink(&user.user.id, &id).await? {
        return Err(AppError::NotFound);
    }
    Ok(ok(json!({ "unlinked": true })))
}
