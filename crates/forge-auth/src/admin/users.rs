use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::db::users::NewUser;
use crate::error::AppError;
use crate::session::AdminUser;
use crate::state::SharedState;
use crate::util::hash_password;

async fn user_json(
    state: &SharedState,
    user: &crate::db::models::User,
) -> Result<serde_json::Value, AppError> {
    let roles = state.db.user_role_names(&user.id).await?;
    let mut value = serde_json::to_value(user).map_err(AppError::internal)?;
    value["roles"] = json!(roles);
    Ok(value)
}

pub async fn list(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.db.users_list().await?;
    let mut out = Vec::with_capacity(users.len());
    for user in &users {
        out.push(user_json(&state, user).await?);
    }
    Ok(ok(out))
}

#[derive(Deserialize)]
pub struct CreateUser {
    pub username: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: bool,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

pub async fn create(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Json(body): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let username = body.username.trim();
    if username.is_empty() {
        return Err(AppError::BadRequest("username is required".into()));
    }
    let user = state
        .db
        .user_create(NewUser {
            username,
            email: body.email.as_deref(),
            email_verified: body.email_verified,
            display_name: body.display_name.as_deref(),
        })
        .await?;
    if let Some(password) = &body.password {
        state
            .db
            .password_set(&user.id, &hash_password(password)?)
            .await?;
    }
    set_roles(&state, &user.id, &body.roles).await?;
    Ok(ok(user_json(&state, &user).await?))
}

pub async fn get(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.db.user_by_id(&id).await?.ok_or(AppError::NotFound)?;
    Ok(ok(user_json(&state, &user).await?))
}

#[derive(Deserialize)]
pub struct UpdateUser {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub disabled: Option<bool>,
}

pub async fn update(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.db.user_by_id(&id).await?.ok_or(AppError::NotFound)?;
    let email = body.email.or(user.email);
    let disabled = body.disabled.unwrap_or(user.disabled);
    state
        .db
        .user_update(
            &id,
            email.as_deref(),
            body.email_verified.unwrap_or(user.email_verified),
            body.display_name.or(user.display_name).as_deref(),
            disabled,
        )
        .await?;
    if disabled {
        state.db.refresh_tokens_revoke_for_user(&id).await?;
    }
    let user = state.db.user_by_id(&id).await?.ok_or(AppError::NotFound)?;
    Ok(ok(user_json(&state, &user).await?))
}

pub async fn delete(
    admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if admin.user.id == id {
        return Err(AppError::BadRequest(
            "refusing to delete the signed-in admin".into(),
        ));
    }
    if !state.db.user_delete(&id).await? {
        return Err(AppError::NotFound);
    }
    Ok(ok(json!({ "deleted": true })))
}

#[derive(Deserialize)]
pub struct SetPassword {
    pub password: String,
}

pub async fn set_password(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<SetPassword>,
) -> Result<impl IntoResponse, AppError> {
    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    state.db.user_by_id(&id).await?.ok_or(AppError::NotFound)?;
    state
        .db
        .password_set(&id, &hash_password(&body.password)?)
        .await?;
    Ok(ok(json!({ "updated": true })))
}

#[derive(Deserialize)]
pub struct SetRoles {
    pub roles: Vec<String>,
}

pub async fn set_user_roles(
    _admin: AdminUser,
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<SetRoles>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.db.user_by_id(&id).await?.ok_or(AppError::NotFound)?;
    set_roles(&state, &id, &body.roles).await?;
    Ok(ok(user_json(&state, &user).await?))
}

/// Resolve role names → ids (rejecting unknown names) and replace the user's
/// manually-assigned roles.
async fn set_roles(
    state: &SharedState,
    user_id: &str,
    role_names: &[String],
) -> Result<(), AppError> {
    let mut role_ids = Vec::with_capacity(role_names.len());
    for name in role_names {
        let role = state
            .db
            .role_by_name(name)
            .await?
            .ok_or_else(|| AppError::BadRequest(format!("unknown role {name:?}")))?;
        role_ids.push(role.id);
    }
    state
        .db
        .user_roles_replace(user_id, &role_ids, "manual")
        .await?;
    Ok(())
}
