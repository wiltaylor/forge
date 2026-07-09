//! HS256 JWT encoding and decoding. The claims shape itself lives in
//! [`forge_core::claims`].

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use crate::error::ForgeError;

pub use forge_core::claims::{unix_now, Claims};

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
