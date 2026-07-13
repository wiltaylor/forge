use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::error::AppError;
use crate::session::AdminUser;
use crate::state::SharedState;

pub async fn list(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ok(state.db.roles_list().await?))
}

#[derive(Deserialize)]
pub struct CreateRole {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Json(body): Json<CreateRole>,
) -> Result<impl IntoResponse, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("role name is required".into()));
    }
    Ok(ok(state
        .db
        .role_create(name, body.description.as_deref())
        .await?))
}

pub async fn delete(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if !state.db.role_delete(&id).await? {
        return Err(AppError::NotFound);
    }
    Ok(ok(json!({ "deleted": true })))
}
