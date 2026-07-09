//! Authentication: HS256 JWTs with a pluggable [`TokenValidator`].
//!
//! Auth-disabled mode is first-class: with no secret configured every
//! endpoint is open and handlers see [`Claims::anonymous`].

pub mod extract;
pub mod jwt;
pub mod routes;
pub mod users;

use std::sync::Arc;

pub use jwt::{decode_token, encode_token, Claims};
pub use users::AuthUser;

use crate::error::ForgeError;

/// Default token lifetime (seconds) — 24 hours.
pub const DEFAULT_TTL_SECS: u64 = 86_400;
/// Default issuer claim.
pub const DEFAULT_ISS: &str = "forge";

/// Auth configuration: HS256 shared secret plus login users.
#[derive(Clone)]
pub struct AuthConfig {
    /// HS256 shared secret. Must be at least 32 characters.
    pub secret: String,
    /// Token lifetime in seconds (default 86400).
    pub ttl_secs: u64,
    /// Issuer claim set on minted tokens (default `"forge"`).
    pub iss: String,
    /// Validate `iss` on incoming tokens. Only enabled when the issuer was
    /// explicitly configured (contract: validated only when configured).
    pub validate_iss: bool,
    /// Users accepted by `POST /api/auth/login`.
    pub users: Vec<AuthUser>,
}

impl AuthConfig {
    /// New config with defaults (ttl 86400, iss "forge" unvalidated, no users).
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            ttl_secs: DEFAULT_TTL_SECS,
            iss: DEFAULT_ISS.to_string(),
            validate_iss: false,
            users: Vec::new(),
        }
    }

    /// Set the token lifetime in seconds.
    pub fn ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    /// Explicitly set the issuer — also enables issuer validation.
    pub fn issuer(mut self, iss: impl Into<String>) -> Self {
        self.iss = iss.into();
        self.validate_iss = true;
        self
    }

    /// Add a login user. `secret` is either a plaintext password or an
    /// argon2 PHC hash (`$argon2...`).
    pub fn user(mut self, name: impl Into<String>, secret: impl Into<String>) -> Self {
        self.users.push(AuthUser::new(name, secret));
        self
    }

    /// Add a login user with roles.
    pub fn user_with_roles(
        mut self,
        name: impl Into<String>,
        secret: impl Into<String>,
        roles: Vec<String>,
    ) -> Self {
        self.users.push(AuthUser::new(name, secret).roles(roles));
        self
    }

    /// Contract: startup fails when the secret is set but shorter than 32 chars.
    pub fn validate(&self) -> Result<(), ForgeError> {
        if self.secret.len() < 32 {
            return Err(ForgeError::Config(
                "FORGE_JWT_SECRET must be at least 32 characters".into(),
            ));
        }
        Ok(())
    }

    /// Build from `FORGE_JWT_SECRET`, `FORGE_AUTH_USERS`, `FORGE_JWT_TTL_SECS`
    /// and `FORGE_JWT_ISS`. Returns `Ok(None)` when no secret is set
    /// (auth-disabled mode).
    pub fn from_env() -> Result<Option<Self>, ForgeError> {
        let Ok(secret) = std::env::var("FORGE_JWT_SECRET") else {
            return Ok(None);
        };
        let mut cfg = AuthConfig::new(secret);
        cfg.validate()?;
        if let Ok(raw) = std::env::var("FORGE_AUTH_USERS") {
            cfg.users = users::parse_users(&raw)?;
        }
        if let Ok(raw) = std::env::var("FORGE_JWT_TTL_SECS") {
            cfg.ttl_secs = raw.parse().map_err(|_| {
                ForgeError::Config(format!("FORGE_JWT_TTL_SECS is not a number: {raw:?}"))
            })?;
        }
        if let Ok(iss) = std::env::var("FORGE_JWT_ISS") {
            cfg = cfg.issuer(iss);
        }
        Ok(Some(cfg))
    }
}

/// Extension point for token validation (e.g. RS256/JWKS). The default is
/// [`Hs256Validator`].
pub trait TokenValidator: Send + Sync {
    fn validate(&self, token: &str) -> Result<Claims, ForgeError>;
}

/// Default validator: HS256 shared secret, optional issuer check.
pub struct Hs256Validator {
    secret: String,
    /// When `Some`, incoming tokens must carry this issuer.
    iss: Option<String>,
}

impl Hs256Validator {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            iss: None,
        }
    }

    pub fn with_issuer(mut self, iss: impl Into<String>) -> Self {
        self.iss = Some(iss.into());
        self
    }
}

impl TokenValidator for Hs256Validator {
    fn validate(&self, token: &str) -> Result<Claims, ForgeError> {
        decode_token(token, &self.secret, self.iss.as_deref())
    }
}

/// Runtime auth state stored in [`crate::state::ForgeState`].
#[derive(Clone)]
pub(crate) struct AuthState {
    pub validator: Arc<dyn TokenValidator>,
    /// `Some` when login (token minting) is available; `None` in external
    /// issuer mode (custom validator without an [`AuthConfig`]).
    pub config: Option<AuthConfig>,
}
