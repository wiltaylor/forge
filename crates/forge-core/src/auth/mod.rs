//! Authentication: HS256 JWTs, the login user store and password
//! verification — all of it transport-free.
//!
//! [`Auth`] is what a transport holds: it validates a token it was handed and
//! mints one for a username and password it was handed. Where those strings
//! came from — an HTTP header, an IPC message — is the transport's business.
//!
//! Auth-disabled mode is first-class: with no [`Auth`] at all every endpoint
//! is open and handlers see [`Claims::anonymous`].

pub mod jwt;
pub mod users;

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::ForgeError;

pub use crate::claims::{unix_now, Claims};
pub use jwt::{decode_token, encode_token};
pub use users::{parse_users, AuthUser};

/// Default token lifetime (seconds) — 24 hours.
pub const DEFAULT_TTL_SECS: u64 = 86_400;
/// Default issuer claim.
pub const DEFAULT_ISS: &str = "forge";
/// Error message for a login attempt while auth is disabled. The contract
/// makes this a 404: with no auth configured there is no login endpoint.
pub const AUTH_DISABLED: &str = "auth is disabled";

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
    /// Users accepted by login.
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
            cfg.users = parse_users(&raw)?;
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

    /// The user record for `username`, but only when `password` verifies.
    pub fn verify_user(&self, username: &str, password: &str) -> Option<&AuthUser> {
        self.users
            .iter()
            .find(|u| u.name == username)
            .filter(|u| u.verify(password))
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

/// The login response body the contract specifies. One definition, so every
/// transport answers a successful login with the same shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    /// The minted JWT.
    pub token: String,
    /// Expiry of `token` (unix seconds) — the `exp` claim.
    pub expires_at: i64,
    pub user: LoginUser,
}

/// The `user` member of [`LoginResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginUser {
    pub name: String,
    pub roles: Vec<String>,
}

/// Runtime auth: a token validator, plus the config that mints tokens when
/// login is available.
///
/// Transports hold an `Option<Auth>`; `None` is auth-disabled mode.
#[derive(Clone)]
pub struct Auth {
    validator: Arc<dyn TokenValidator>,
    /// `Some` when login (token minting) is available; `None` in external
    /// issuer mode (custom validator without an [`AuthConfig`]).
    config: Option<AuthConfig>,
}

impl Auth {
    /// Auth backed by the stock HS256 validator built from `config`.
    pub fn new(config: AuthConfig) -> Self {
        let mut validator = Hs256Validator::new(config.secret.clone());
        if config.validate_iss {
            validator = validator.with_issuer(config.iss.clone());
        }
        Self {
            validator: Arc::new(validator),
            config: Some(config),
        }
    }

    /// Auth backed by a caller-supplied validator (e.g. RS256/JWKS). Login
    /// stays available only when a [`AuthConfig`] comes with it.
    pub fn with_validator(validator: Arc<dyn TokenValidator>, config: Option<AuthConfig>) -> Self {
        Self { validator, config }
    }

    /// Whether this instance can mint tokens. `false` in external-issuer
    /// mode, where there is no login endpoint to offer.
    pub fn can_login(&self) -> bool {
        self.config.is_some()
    }

    /// Validate a token and return its claims.
    pub fn validate(&self, token: &str) -> Result<Claims, ForgeError> {
        self.validator.validate(token)
    }

    /// Check a username and password and mint a token for them.
    ///
    /// `NotFound` when this instance cannot mint tokens (external-issuer
    /// mode), `Unauthorized` when the credentials do not verify.
    pub fn login(&self, username: &str, password: &str) -> Result<LoginResponse, ForgeError> {
        let Some(config) = &self.config else {
            return Err(ForgeError::NotFound(AUTH_DISABLED.into()));
        };
        let Some(user) = config.verify_user(username, password) else {
            return Err(ForgeError::Unauthorized(
                "invalid username or password".into(),
            ));
        };
        let claims = Claims::new(
            &user.name,
            user.roles.clone(),
            config.ttl_secs,
            Some(config.iss.clone()),
        );
        Ok(LoginResponse {
            token: encode_token(&claims, &config.secret)?,
            expires_at: claims.exp,
            user: LoginUser {
                name: user.name.clone(),
                roles: user.roles.clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claims::unix_now;

    const SECRET: &str = "0123456789abcdef0123456789abcdef"; // 32 chars

    fn auth() -> Auth {
        Auth::new(
            AuthConfig::new(SECRET)
                .user("admin", "hunter2")
                .user_with_roles("ops", "opspass", vec!["ops".into(), "admin".into()]),
        )
    }

    #[test]
    fn login_mints_a_token_the_same_auth_validates() {
        let auth = auth();
        let response = auth.login("admin", "hunter2").unwrap();
        assert_eq!(response.user.name, "admin");
        assert!(response.user.roles.is_empty());
        assert!((response.expires_at - (unix_now() + 86_400)).abs() < 10);
        let claims = auth.validate(&response.token).unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.exp, response.expires_at);
        assert_eq!(claims.iss.as_deref(), Some(DEFAULT_ISS));
    }

    #[test]
    fn login_carries_roles_into_the_token() {
        let auth = auth();
        let response = auth.login("ops", "opspass").unwrap();
        assert_eq!(response.user.roles, vec!["ops", "admin"]);
        let claims = auth.validate(&response.token).unwrap();
        assert_eq!(claims.roles, vec!["ops", "admin"]);
    }

    #[test]
    fn wrong_password_and_unknown_user_are_both_401() {
        let auth = auth();
        assert_eq!(auth.login("admin", "wrong").unwrap_err().status(), 401);
        assert_eq!(auth.login("ghost", "hunter2").unwrap_err().status(), 401);
    }

    #[test]
    fn login_verifies_an_argon2_hashed_secret() {
        // Real forge-hash output for "s3cret".
        let hash = "$argon2id$v=19$m=19456,t=2,p=1$oIonqyWQKyCxWwPHZhDWFQ$\
                    MatG+SWES3ScVBSWxHLZ6y0BwSH/jxcJYuHBHO3gME4";
        let auth = Auth::new(AuthConfig::new(SECRET).user("h", hash));
        assert!(auth.login("h", "s3cret").is_ok());
        assert_eq!(auth.login("h", "nope").unwrap_err().status(), 401);
    }

    #[test]
    fn issuer_is_validated_only_when_configured() {
        let plain = auth();
        let checked = Auth::new(AuthConfig::new(SECRET).issuer("my-issuer").user("a", "b"));
        // A token carrying the stock issuer passes the unconfigured auth...
        let token = plain.login("admin", "hunter2").unwrap().token;
        assert!(plain.validate(&token).is_ok());
        // ...and fails the one that requires its own issuer.
        assert_eq!(checked.validate(&token).unwrap_err().status(), 401);
        let mine = checked.login("a", "b").unwrap().token;
        assert_eq!(
            checked.validate(&mine).unwrap().iss.as_deref(),
            Some("my-issuer")
        );
    }

    #[test]
    fn external_issuer_mode_has_no_login() {
        let validator = Arc::new(Hs256Validator::new(SECRET)) as Arc<dyn TokenValidator>;
        let auth = Auth::with_validator(validator, None);
        let err = auth.login("admin", "hunter2").unwrap_err();
        assert_eq!(err.status(), 404);
        assert_eq!(err.to_string(), AUTH_DISABLED);
        // Validation still works — that is the whole point of the mode.
        let token = encode_token(&Claims::new("x", vec![], 3600, None), SECRET).unwrap();
        assert_eq!(auth.validate(&token).unwrap().sub, "x");
    }

    #[test]
    fn a_short_secret_is_a_config_error() {
        assert!(AuthConfig::new("too-short").validate().is_err());
        assert!(AuthConfig::new(SECRET).validate().is_ok());
    }
}
