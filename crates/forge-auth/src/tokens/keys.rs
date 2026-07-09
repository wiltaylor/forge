//! RS256 signing keys: generation, persistence, JWKS.

use jsonwebtoken::{DecodingKey, EncodingKey};
use rsa::pkcs8::{DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::traits::PublicKeyParts;
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};

use crate::db::models::SigningKey;
use crate::db::Db;
use crate::error::AppError;
use crate::util::{b64url, new_id, now};

/// Retired keys stay in the JWKS for this long, covering in-flight tokens.
pub const RETIRED_KEY_GRACE: i64 = 24 * 3600;

pub struct KeySet {
    pub active_kid: String,
    pub encoding: EncodingKey,
    /// kid → decoder for every published (active or in-grace retired) key.
    pub decoders: Vec<(String, DecodingKey)>,
    pub jwks: Value,
}

impl KeySet {
    pub fn decoder(&self, kid: &str) -> Option<&DecodingKey> {
        self.decoders.iter().find(|(k, _)| k == kid).map(|(_, d)| d)
    }
}

fn generate_key() -> Result<SigningKey, AppError> {
    let private = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).map_err(AppError::internal)?;
    let private_pem = private
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(AppError::internal)?
        .to_string();
    let public_pem = private
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .map_err(AppError::internal)?;
    Ok(SigningKey {
        kid: new_id(),
        alg: "RS256".into(),
        private_pem,
        public_pem,
        status: "active".into(),
        created_at: now(),
        retired_at: None,
    })
}

fn jwk_for(key: &SigningKey) -> Result<Value, AppError> {
    let public = RsaPublicKey::from_public_key_pem(&key.public_pem).map_err(AppError::internal)?;
    Ok(json!({
        "kty": "RSA",
        "use": "sig",
        "alg": key.alg,
        "kid": key.kid,
        "n": b64url(&public.n().to_bytes_be()),
        "e": b64url(&public.e().to_bytes_be()),
    }))
}

fn build_keyset(all: &[SigningKey]) -> Result<KeySet, AppError> {
    let active = all
        .iter()
        .find(|k| k.status == "active")
        .ok_or_else(|| AppError::Internal("no active signing key".into()))?;
    let cutoff = now() - RETIRED_KEY_GRACE;
    let published: Vec<&SigningKey> = all
        .iter()
        .filter(|k| k.status == "active" || k.retired_at.map(|t| t > cutoff).unwrap_or(false))
        .collect();

    let encoding =
        EncodingKey::from_rsa_pem(active.private_pem.as_bytes()).map_err(AppError::internal)?;
    let mut decoders = Vec::new();
    let mut jwks_keys = Vec::new();
    for key in &published {
        decoders.push((
            key.kid.clone(),
            DecodingKey::from_rsa_pem(key.public_pem.as_bytes()).map_err(AppError::internal)?,
        ));
        jwks_keys.push(jwk_for(key)?);
    }
    Ok(KeySet {
        active_kid: active.kid.clone(),
        encoding,
        decoders,
        jwks: json!({ "keys": jwks_keys }),
    })
}

/// Load keys from the DB, generating the first one on a fresh install.
pub async fn ensure_keys(db: &Db) -> Result<KeySet, AppError> {
    let mut all = db.signing_keys_all().await?;
    if !all.iter().any(|k| k.status == "active") {
        tracing::info!("no active signing key, generating a 2048-bit RSA key");
        let key = generate_key()?;
        db.signing_key_insert(&key).await?;
        all = db.signing_keys_all().await?;
    }
    build_keyset(&all)
}

/// Retire the active key and mint a fresh one. Returns the new key set.
pub async fn rotate(db: &Db) -> Result<KeySet, AppError> {
    db.signing_keys_retire_active().await?;
    let key = generate_key()?;
    db.signing_key_insert(&key).await?;
    db.signing_keys_prune(RETIRED_KEY_GRACE).await?;
    build_keyset(&db.signing_keys_all().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_roundtrips_through_jwk() {
        let key = generate_key().unwrap();
        let jwk = jwk_for(&key).unwrap();
        assert_eq!(jwk["kty"], "RSA");
        assert_eq!(jwk["alg"], "RS256");
        // 2048-bit modulus → 256 bytes → 342 base64url chars, leading byte non-zero.
        let n = jwk["n"].as_str().unwrap();
        assert_eq!(n.len(), 342);
        let set = build_keyset(std::slice::from_ref(&key)).unwrap();
        assert_eq!(set.active_kid, key.kid);
        assert!(set.decoder(&key.kid).is_some());
    }
}
