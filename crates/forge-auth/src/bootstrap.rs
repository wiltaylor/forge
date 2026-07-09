//! First-boot provisioning: the `admin` role, the bootstrap admin user, and
//! optional seed-file upserts (providers/clients) for reproducible deploys.

use serde::Deserialize;

use crate::db::users::NewUser;
use crate::error::AppError;
use crate::state::SharedState;
use crate::util::{hash_password, random_token};

pub async fn run(state: &SharedState) -> Result<(), AppError> {
    let admin_role = state.db.role_ensure("admin", Some("forge-auth administration")).await?;

    if state.db.user_count().await? == 0 {
        let username = state.cfg.admin_user.clone();
        let password = match &state.cfg.admin_password {
            Some(p) => p.clone(),
            None => {
                let generated = random_token("");
                tracing::warn!(
                    username,
                    password = %generated,
                    "no FORGE_AUTH_ADMIN_PASSWORD set — generated a one-time admin password (shown once)"
                );
                generated
            }
        };
        let user = state
            .db
            .user_create(NewUser {
                username: &username,
                email: None,
                email_verified: false,
                display_name: Some("Administrator"),
            })
            .await?;
        state.db.password_set(&user.id, &hash_password(&password)?).await?;
        state.db.user_role_add(&user.id, &admin_role.id, "manual").await?;
        tracing::info!(username, "bootstrap admin user created");
    }

    if let Some(path) = &state.cfg.seed_file {
        seed_from_file(state, path).await?;
    }

    #[cfg(feature = "dev-login")]
    crate::login::dev::seed(state).await?;

    Ok(())
}

#[derive(Deserialize)]
struct SeedFile {
    #[serde(default)]
    providers: Vec<SeedProvider>,
    #[serde(default)]
    roles: Vec<SeedRole>,
}

#[derive(Deserialize)]
struct SeedRole {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct SeedProvider {
    slug: String,
    kind: String,
    display_name: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_true")]
    allow_signup: bool,
    #[serde(default)]
    link_by_email: bool,
    #[serde(default)]
    config: Option<toml::Value>,
}

fn default_true() -> bool {
    true
}

async fn seed_from_file(state: &SharedState, path: &str) -> Result<(), AppError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| AppError::Config(format!("cannot read seed file {path:?}: {e}")))?;
    let seed: SeedFile =
        toml::from_str(&raw).map_err(|e| AppError::Config(format!("bad seed file: {e}")))?;

    for role in &seed.roles {
        state.db.role_ensure(&role.name, role.description.as_deref()).await?;
    }
    for provider in &seed.providers {
        let config = match &provider.config {
            Some(value) => serde_json::to_value(value).map_err(AppError::internal)?,
            None => serde_json::json!({}),
        };
        state
            .db
            .provider_upsert(
                &provider.slug,
                &provider.kind,
                &provider.display_name,
                provider.enabled,
                provider.allow_signup,
                provider.link_by_email,
                &config,
            )
            .await?;
        tracing::info!(slug = %provider.slug, kind = %provider.kind, "seeded upstream provider");
    }
    Ok(())
}
