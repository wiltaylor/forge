use axum::response::IntoResponse;
use axum::Extension;
use serde_json::json;

use crate::api::ok;
use crate::error::AppError;
use crate::session::AdminUser;
use crate::state::SharedState;
use crate::tokens::keys;

pub async fn list(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let all = state.db.signing_keys_all().await?;
    let out: Vec<_> = all
        .iter()
        .map(|k| {
            json!({
                "kid": k.kid,
                "alg": k.alg,
                "status": k.status,
                "created_at": k.created_at,
                "retired_at": k.retired_at,
            })
        })
        .collect();
    Ok(ok(out))
}

pub async fn rotate(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let new_set = keys::rotate(&state.db).await?;
    let kid = new_set.active_kid.clone();
    *state.keys.write().await = new_set;
    tracing::info!(kid, "signing key rotated");
    Ok(ok(json!({ "active_kid": kid })))
}
