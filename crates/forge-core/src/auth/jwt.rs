//! HS256 JWT encoding and decoding. The claims shape itself lives in
//! [`crate::claims`].

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

use crate::error::ForgeError;

pub use crate::claims::{unix_now, Claims};

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

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef"; // 32 chars

    #[test]
    fn round_trip_carries_sub_and_roles() {
        let claims = Claims::new("admin", vec!["ops".into()], 3600, None);
        let token = encode_token(&claims, SECRET).unwrap();
        let decoded = decode_token(&token, SECRET, None).unwrap();
        assert_eq!(decoded.sub, "admin");
        assert_eq!(decoded.roles, vec!["ops".to_string()]);
        assert_eq!(decoded.exp, claims.exp);
    }

    #[test]
    fn expired_token_is_unauthorized() {
        let now = unix_now();
        let claims = Claims {
            sub: "admin".into(),
            roles: vec![],
            iat: now - 7200,
            exp: now - 3600,
            iss: None,
        };
        let token = encode_token(&claims, SECRET).unwrap();
        let err = decode_token(&token, SECRET, None).unwrap_err();
        assert_eq!(err.status(), 401);
    }

    #[test]
    fn wrong_secret_is_unauthorized() {
        let token = encode_token(&Claims::new("a", vec![], 3600, None), SECRET).unwrap();
        let err = decode_token(&token, "another-secret-another-secret-32", None).unwrap_err();
        assert_eq!(err.status(), 401);
    }

    #[test]
    fn issuer_checked_only_when_requested() {
        let claims = Claims::new("a", vec![], 3600, Some("mine".into()));
        let token = encode_token(&claims, SECRET).unwrap();
        assert!(decode_token(&token, SECRET, None).is_ok());
        assert!(decode_token(&token, SECRET, Some("mine")).is_ok());
        assert!(decode_token(&token, SECRET, Some("other")).is_err());
    }

    #[test]
    fn token_without_exp_is_rejected() {
        // `exp` is a required spec claim: a token that omits it cannot expire.
        let token = encode(
            &Header::new(Algorithm::HS256),
            &serde_json::json!({"sub": "a", "roles": [], "iat": unix_now()}),
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        assert!(decode_token(&token, SECRET, None).is_err());
    }
}
