use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Extension;
use serde_json::json;

use crate::api::ok;
use crate::error::AppError;
use crate::session::AdminUser;
use crate::state::SharedState;

pub async fn list(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let sessions = state.db.sessions_list_active().await?;
    let mut out = Vec::with_capacity(sessions.len());
    for s in sessions {
        let user = state.db.user_by_id(&s.user_id).await?;
        out.push(json!({
            "id": s.id_hash,
            "user_id": s.user_id,
            "username": user.map(|u| u.username),
            "amr": s.amr,
            "created_at": s.created_at,
            "last_seen": s.last_seen,
            "expires_at": s.expires_at,
        }));
    }
    Ok(ok(out))
}

pub async fn revoke(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if !state.db.session_revoke(&id).await? {
        return Err(AppError::NotFound);
    }
    Ok(ok(json!({ "revoked": true })))
}
