//! forge-auth: OIDC identity provider for the Forge ecosystem.

pub mod admin;
pub mod api;
pub mod app;
pub mod bootstrap;
pub mod config;
pub mod db;
pub mod error;
pub mod login;
pub mod me;
pub mod oidc;
pub mod ratelimit;
pub mod session;
pub mod state;
pub mod tokens;
pub mod upstream;
pub mod util;

use std::sync::Arc;

use config::Config;
use error::AppError;
use state::{AppState, SharedState};

/// Connect, migrate, bootstrap and return shared state.
pub async fn init_state(cfg: Config) -> Result<SharedState, AppError> {
    let db = db::Db::connect(&cfg.database_url).await?;
    let keys = tokens::keys::ensure_keys(&db).await?;
    let state: SharedState = Arc::new(AppState {
        db,
        cfg,
        keys: tokio::sync::RwLock::new(keys),
        http: reqwest::Client::builder()
            .user_agent("forge-auth")
            // openidconnect requires a non-redirecting client (SSRF hygiene).
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(AppError::internal)?,
        login_limiter: ratelimit::LoginLimiter::default(),
    });
    bootstrap::run(&state).await?;
    Ok(state)
}

/// Hourly cleanup of expired sessions/codes/requests/tokens.
pub fn spawn_sweeper(state: SharedState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            tick.tick().await;
            if let Err(e) = state.db.sweep_expired().await {
                tracing::warn!(error = %e, "expired-row sweep failed");
            }
            if let Err(e) = state
                .db
                .signing_keys_prune(tokens::keys::RETIRED_KEY_GRACE)
                .await
            {
                tracing::warn!(error = %e, "signing-key prune failed");
            }
        }
    });
}
