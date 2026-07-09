//! All runtime configuration, read once from the environment at startup.

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Config {
    /// Public base URL of this IdP (no trailing slash). Discovery, token `iss`
    /// and redirect targets all derive from it.
    pub issuer: String,
    pub database_url: String,
    pub cookie_secure: bool,
    pub access_ttl: i64,
    pub refresh_ttl: i64,
    pub session_idle_ttl: i64,
    pub session_absolute_ttl: i64,
    pub admin_user: String,
    pub admin_password: Option<String>,
    pub seed_file: Option<String>,
    pub host: String,
    pub port: u16,
}

fn env_i64(key: &str, default: i64) -> Result<i64, AppError> {
    match std::env::var(key) {
        Ok(raw) => raw
            .parse()
            .map_err(|_| AppError::Config(format!("{key} is not a number: {raw:?}"))),
        Err(_) => Ok(default),
    }
}

fn env_bool(key: &str, default: bool) -> Result<bool, AppError> {
    match std::env::var(key) {
        Ok(raw) => match raw.as_str() {
            "1" | "true" | "yes" => Ok(true),
            "0" | "false" | "no" => Ok(false),
            _ => Err(AppError::Config(format!("{key} is not a bool: {raw:?}"))),
        },
        Err(_) => Ok(default),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let host = std::env::var("FORGE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = env_i64("FORGE_PORT", 8770)? as u16;
        let issuer = match std::env::var("FORGE_AUTH_ISSUER") {
            Ok(raw) => raw.trim_end_matches('/').to_string(),
            Err(_) => {
                let fallback = format!("http://127.0.0.1:{port}");
                tracing::warn!("FORGE_AUTH_ISSUER unset, defaulting to {fallback} (dev only)");
                fallback
            }
        };
        Ok(Self {
            issuer,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/forge-auth.db?mode=rwc".into()),
            cookie_secure: env_bool("FORGE_AUTH_COOKIE_SECURE", true)?,
            access_ttl: env_i64("FORGE_AUTH_ACCESS_TTL", 900)?,
            refresh_ttl: env_i64("FORGE_AUTH_REFRESH_TTL", 30 * 24 * 3600)?,
            session_idle_ttl: env_i64("FORGE_AUTH_SESSION_IDLE_TTL", 12 * 3600)?,
            session_absolute_ttl: env_i64("FORGE_AUTH_SESSION_ABSOLUTE_TTL", 7 * 24 * 3600)?,
            admin_user: std::env::var("FORGE_AUTH_ADMIN_USER").unwrap_or_else(|_| "admin".into()),
            admin_password: std::env::var("FORGE_AUTH_ADMIN_PASSWORD").ok(),
            seed_file: std::env::var("FORGE_AUTH_SEED_FILE").ok(),
            host,
            port,
        })
    }
}
