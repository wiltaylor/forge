//! HS256 JWT claims, encoding and decoding.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::ForgeError;

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

/// Encode claims as an HS256 JWT.
pub fn encode_token(claims: &Claims, secret: &str) -> Result<String, ForgeError> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ForgeError::Internal(format!("failed to encode token: {e}")))
}

/// Decode and validate an HS256 JWT. `iss` is checked only when `Some`.
pub fn decode_token(token: &str, secret: &str, iss: Option<&str>) -> Result<Claims, ForgeError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_required_spec_claims(&["exp"]);
    if let Some(iss) = iss {
        validation.set_issuer(&[iss]);
    }
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| ForgeError::Unauthorized(format!("invalid token: {e}")))
}
