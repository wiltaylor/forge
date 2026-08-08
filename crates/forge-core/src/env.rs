//! The contract's environment variables (docs/api-contract.md, "Environment
//! variables"), each read by exactly one function here, with its default
//! stated once.
//!
//! Other `FORGE_*` names in this repo — the widget switches
//! (`FORGE_TERM_*`, `FORGE_VNC_ENABLE`, `FORGE_RDP_ENABLE`,
//! `FORGE_DESKTOP_ALLOW_HOSTS`), the `FORGE_TEST_*` variables, the demo
//! and gallery variables, and the `forge-auth` peer service's
//! `FORGE_AUTH_*` configuration — are not part of the contract and are
//! not read here.

use crate::error::ForgeError;

/// Default token lifetime (seconds) — 24 hours.
pub const DEFAULT_TTL_SECS: u64 = 86_400;
/// Default issuer claim.
pub const DEFAULT_ISS: &str = "forge";
/// Default bind host.
pub const DEFAULT_HOST: &str = "127.0.0.1";
/// Default bind port.
pub const DEFAULT_PORT: u16 = 8765;
/// Default doc-store directory.
pub const DEFAULT_DATA_DIR: &str = "./data";
/// Default component-federation directory.
pub const DEFAULT_COMPONENTS_DIR: &str = "./components";
/// Default CORS origin allowlist (comma-separated).
pub const DEFAULT_CORS_ORIGINS: &str = "http://localhost:5173,http://127.0.0.1:5173";

/// `FORGE_JWT_SECRET` — no default; unset means auth-disabled mode.
pub fn jwt_secret() -> Option<String> {
    std::env::var("FORGE_JWT_SECRET").ok()
}

/// `FORGE_AUTH_USERS`, raw — parse with [`crate::auth::parse_users`].
pub fn auth_users() -> Option<String> {
    std::env::var("FORGE_AUTH_USERS").ok()
}

/// `FORGE_JWT_TTL_SECS` (default [`DEFAULT_TTL_SECS`]).
pub fn jwt_ttl_secs() -> Result<u64, ForgeError> {
    match std::env::var("FORGE_JWT_TTL_SECS") {
        Ok(raw) => raw.parse().map_err(|_| {
            ForgeError::Config(format!("FORGE_JWT_TTL_SECS is not a number: {raw:?}"))
        }),
        Err(_) => Ok(DEFAULT_TTL_SECS),
    }
}

/// `FORGE_JWT_ISS` — `None` when unset.
///
/// [`DEFAULT_ISS`] is not applied here on purpose: the contract validates
/// the issuer only when it is set explicitly, so the caller must see the
/// difference between unset and `"forge"`.
pub fn jwt_iss() -> Option<String> {
    std::env::var("FORGE_JWT_ISS").ok()
}

/// `FORGE_HOST` (default [`DEFAULT_HOST`]).
pub fn host() -> String {
    std::env::var("FORGE_HOST").unwrap_or_else(|_| DEFAULT_HOST.into())
}

/// `FORGE_PORT` (default [`DEFAULT_PORT`]).
pub fn port() -> Result<u16, ForgeError> {
    match std::env::var("FORGE_PORT") {
        Ok(raw) => raw
            .parse()
            .map_err(|_| ForgeError::Config(format!("FORGE_PORT is not a port: {raw:?}"))),
        Err(_) => Ok(DEFAULT_PORT),
    }
}

/// `FORGE_DATA_DIR` (default [`DEFAULT_DATA_DIR`]).
pub fn data_dir() -> String {
    std::env::var("FORGE_DATA_DIR").unwrap_or_else(|_| DEFAULT_DATA_DIR.into())
}

/// `FORGE_COMPONENTS_DIR` (default [`DEFAULT_COMPONENTS_DIR`]).
pub fn components_dir() -> String {
    std::env::var("FORGE_COMPONENTS_DIR").unwrap_or_else(|_| DEFAULT_COMPONENTS_DIR.into())
}

/// `FORGE_CORS_ORIGINS`, split on commas (default [`DEFAULT_CORS_ORIGINS`]).
pub fn cors_origins() -> Vec<String> {
    split_origins(&std::env::var("FORGE_CORS_ORIGINS").unwrap_or_else(|_| {
        DEFAULT_CORS_ORIGINS.into()
    }))
}

fn split_origins(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins every default to the literal value docs/api-contract.md
    /// documents, so this backend cannot drift from the contract (or from
    /// the Python backend, whose test_config.py pins the same values)
    /// while this test is green.
    #[test]
    fn defaults_match_the_contract_table() {
        assert_eq!(DEFAULT_TTL_SECS, 86_400);
        assert_eq!(DEFAULT_ISS, "forge");
        assert_eq!(DEFAULT_HOST, "127.0.0.1");
        assert_eq!(DEFAULT_PORT, 8765);
        assert_eq!(DEFAULT_DATA_DIR, "./data");
        assert_eq!(DEFAULT_COMPONENTS_DIR, "./components");
        assert_eq!(
            DEFAULT_CORS_ORIGINS,
            "http://localhost:5173,http://127.0.0.1:5173"
        );
    }

    #[test]
    fn origins_split_on_commas_and_trim() {
        assert_eq!(
            split_origins("https://a.example, https://b.example ,"),
            vec!["https://a.example", "https://b.example"]
        );
        assert_eq!(
            split_origins(DEFAULT_CORS_ORIGINS),
            vec!["http://localhost:5173", "http://127.0.0.1:5173"]
        );
    }
}
