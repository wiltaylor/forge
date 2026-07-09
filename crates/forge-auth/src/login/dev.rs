//! Password-less development login. This module only exists when the
//! `dev-login` cargo feature is compiled in; a production binary has neither
//! these routes nor the seeded users.

use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::json;

use crate::api::ok;
use crate::db::users::NewUser;
use crate::error::AppError;
use crate::session::start_session;
use crate::state::SharedState;

const DEV_USERS: &[(&str, &str, &[&str])] = &[
    ("dev-admin", "Dev Admin", &["admin"]),
    ("dev-user", "Dev User", &["user"]),
    ("dev-viewer", "Dev Viewer", &["viewer"]),
];

/// Seed the selectable dev users (idempotent), called at startup.
pub async fn seed(state: &SharedState) -> Result<(), AppError> {
    tracing::warn!("dev-login is compiled in — DO NOT USE THIS BUILD IN PRODUCTION");
    for (username, display, roles) in DEV_USERS {
        let user = match state.db.user_by_username(username).await? {
            Some(u) => u,
            None => {
                state
                    .db
                    .user_create(NewUser {
                        username,
                        email: Some(&format!("{username}@dev.local")),
                        email_verified: true,
                        display_name: Some(display),
                    })
                    .await?
            }
        };
        for role_name in *roles {
            let role = state.db.role_ensure(role_name, Some("dev-login seed")).await?;
            state.db.user_role_add(&user.id, &role.id, "manual").await?;
        }
    }
    Ok(())
}

pub async fn list_users(
    Extension(state): Extension<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let mut users = Vec::new();
    for (username, display, roles) in DEV_USERS {
        if let Some(user) = state.db.user_by_username(username).await? {
            users.push(json!({
                "id": user.id,
                "username": username,
                "display_name": display,
                "roles": roles,
            }));
        }
    }
    Ok(ok(json!({ "users": users })))
}

#[derive(Deserialize)]
pub struct DevLoginBody {
    pub user_id: String,
    #[serde(default)]
    pub request_id: Option<String>,
}

pub async fn login(
    Extension(state): Extension<SharedState>,
    Json(body): Json<DevLoginBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .db
        .user_by_id(&body.user_id)
        .await?
        .filter(|u| !u.disabled && DEV_USERS.iter().any(|(name, ..)| *name == u.username))
        .ok_or(AppError::NotFound)?;
    let cookie = start_session(&state, &user.id, &["dev".to_string()]).await?;
    let redirect_to = super::post_login_redirect(&state, body.request_id.as_deref()).await?;
    let jar = CookieJar::new().add(cookie);
    Ok((
        jar,
        ok(json!({
            "redirect_to": redirect_to,
            "user": { "id": user.id, "username": user.username },
        })),
    ))
}
