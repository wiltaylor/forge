//! Forge identity claims. JWT encoding/decoding lives with the HTTP server;
//! the claims shape itself is transport-agnostic.

use serde::{Deserialize, Serialize};

/// Forge JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Username.
    pub sub: String,
    /// Role names (default `[]`).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds) = `iat` + TTL.
    pub exp: i64,
    /// Issuer (default `"forge"`); validated only when explicitly configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
}

impl Claims {
    /// The identity handlers see when auth is disabled:
    /// `sub = "anonymous"`, `roles = []`.
    pub fn anonymous() -> Self {
        let now = unix_now();
        Self {
            sub: "anonymous".to_string(),
            roles: Vec::new(),
            iat: now,
            // Far-future expiry; anonymous claims are never wire tokens.
            exp: now + 10 * 365 * 24 * 3600,
            iss: None,
        }
    }

    /// Fresh claims for `sub`, valid for `ttl_secs` from now.
    pub fn new(
        sub: impl Into<String>,
        roles: Vec<String>,
        ttl_secs: u64,
        iss: Option<String>,
    ) -> Self {
        let now = unix_now();
        Self {
            sub: sub.into(),
            roles,
            iat: now,
            exp: now + ttl_secs as i64,
            iss,
        }
    }

    /// True for the auth-disabled anonymous identity.
    pub fn is_anonymous(&self) -> bool {
        self.sub == "anonymous"
    }
}

/// Current unix time in seconds.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
