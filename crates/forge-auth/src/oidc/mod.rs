//! The OAuth2/OIDC protocol surface (`/oauth2/*`, `/.well-known/*`).
//! Everything here speaks raw spec JSON, not the forge envelope.

pub mod authorize;
pub mod client_auth;
pub mod discovery;
pub mod end_session;
pub mod introspect;
pub mod revoke;
pub mod token;
pub mod userinfo;

/// Scopes this IdP understands.
pub const SUPPORTED_SCOPES: &[&str] = &["openid", "profile", "email", "roles", "offline_access"];

pub fn scope_is_supported(scope: &str) -> bool {
    scope.split_whitespace().all(|s| SUPPORTED_SCOPES.contains(&s))
}

pub fn scope_allowed_for_client(scope: &str, allowed: &[String]) -> bool {
    scope
        .split_whitespace()
        .all(|s| allowed.iter().any(|a| a == s) || s == "offline_access")
}
