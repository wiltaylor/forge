//! Access / ID token minting and validation (RS256; HS256 only for the
//! legacy-forge escape hatch).

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::keys::KeySet;
use crate::db::models::{Client, User};
use crate::error::AppError;
use crate::util::now;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azp: Option<String>,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub amr: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub auth_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub amr: Vec<String>,
}

/// Map the IdP's role names into what the client should see. `None` mapping
/// passes everything through; otherwise unmapped roles are dropped.
pub fn map_roles(client: &Client, roles: &[String]) -> Vec<String> {
    match &client.role_mappings {
        None => roles.to_vec(),
        Some(mappings) => roles
            .iter()
            .filter_map(|r| mappings.get(r).and_then(|v| v.as_str()).map(String::from))
            .collect(),
    }
}

fn claims_flag(client: &Client, key: &str) -> bool {
    client
        .claims_config
        .as_ref()
        .and_then(|c| c.get(key))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

pub struct Mint<'a> {
    pub user: &'a User,
    pub client: &'a Client,
    /// Unmapped IdP role names; mapping is applied here.
    pub roles: &'a [String],
    pub scope: &'a str,
    pub amr: &'a [String],
    pub auth_time: i64,
    pub nonce: Option<&'a str>,
    /// `azp` when the token was minted for a different requesting party
    /// (token exchange).
    pub azp: Option<&'a str>,
}

fn scope_has(scope: &str, item: &str) -> bool {
    scope.split_whitespace().any(|s| s == item)
}

pub fn sign_access(
    keys: &KeySet,
    issuer: &str,
    default_ttl: i64,
    mint: &Mint<'_>,
) -> Result<(String, i64), AppError> {
    let ts = now();
    let ttl = mint.client.access_token_ttl.unwrap_or(default_ttl);
    let include_email = claims_flag(mint.client, "include_email") && scope_has(mint.scope, "email");
    let include_profile =
        claims_flag(mint.client, "include_profile") && scope_has(mint.scope, "profile");
    let claims = AccessClaims {
        iss: issuer.to_string(),
        sub: mint.user.id.clone(),
        aud: mint.client.id.clone(),
        azp: mint.azp.map(String::from),
        exp: ts + ttl,
        iat: ts,
        jti: crate::util::new_id(),
        scope: mint.scope.to_string(),
        preferred_username: Some(mint.user.username.clone()),
        email: include_email.then(|| mint.user.email.clone()).flatten(),
        name: include_profile
            .then(|| mint.user.display_name.clone())
            .flatten(),
        roles: map_roles(mint.client, mint.roles),
        amr: mint.amr.to_vec(),
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(keys.active_kid.clone());
    let token = jsonwebtoken::encode(&header, &claims, &keys.encoding).map_err(AppError::internal)?;
    Ok((token, ttl))
}

pub fn sign_id_token(
    keys: &KeySet,
    issuer: &str,
    default_ttl: i64,
    mint: &Mint<'_>,
) -> Result<String, AppError> {
    let ts = now();
    let ttl = mint.client.access_token_ttl.unwrap_or(default_ttl);
    let include_email = claims_flag(mint.client, "include_email") && scope_has(mint.scope, "email");
    let include_profile =
        claims_flag(mint.client, "include_profile") && scope_has(mint.scope, "profile");
    let claims = IdClaims {
        iss: issuer.to_string(),
        sub: mint.user.id.clone(),
        aud: mint.client.id.clone(),
        exp: ts + ttl,
        iat: ts,
        auth_time: mint.auth_time,
        nonce: mint.nonce.map(String::from),
        preferred_username: Some(mint.user.username.clone()),
        email: include_email.then(|| mint.user.email.clone()).flatten(),
        email_verified: include_email.then_some(mint.user.email_verified),
        name: include_profile
            .then(|| mint.user.display_name.clone())
            .flatten(),
        roles: map_roles(mint.client, mint.roles),
        amr: mint.amr.to_vec(),
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(keys.active_kid.clone());
    jsonwebtoken::encode(&header, &claims, &keys.encoding).map_err(AppError::internal)
}

/// Legacy escape hatch: HS256 token in forge-server's stock claim shape so
/// unmodified forge apps accept it with their shared secret.
pub fn sign_legacy_hs256(
    secret: &str,
    issuer: &str,
    username: &str,
    roles: &[String],
    ttl: i64,
) -> Result<String, AppError> {
    #[derive(Serialize)]
    struct LegacyClaims<'a> {
        sub: &'a str,
        roles: &'a [String],
        iat: i64,
        exp: i64,
        iss: &'a str,
    }
    let ts = now();
    let claims = LegacyClaims { sub: username, roles, iat: ts, exp: ts + ttl, iss: issuer };
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AppError::internal)
}

/// Validate one of our own RS256 access tokens. Audience is NOT checked here —
/// callers decide which audiences they accept.
pub fn validate_access(keys: &KeySet, issuer: &str, token: &str) -> Result<AccessClaims, AppError> {
    let header = jsonwebtoken::decode_header(token).map_err(|_| AppError::Unauthorized)?;
    if header.alg != Algorithm::RS256 {
        return Err(AppError::Unauthorized);
    }
    let decoder: &DecodingKey = match header.kid.as_deref().and_then(|kid| keys.decoder(kid)) {
        Some(d) => d,
        None => return Err(AppError::Unauthorized),
    };
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[issuer]);
    validation.validate_aud = false;
    jsonwebtoken::decode::<AccessClaims>(token, decoder, &validation)
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn client(mappings: Option<serde_json::Value>) -> Client {
        Client {
            id: "app".into(),
            name: "App".into(),
            client_type: "confidential".into(),
            secret_hash: None,
            redirect_uris: vec![],
            post_logout_redirect_uris: vec![],
            allowed_scopes: vec![],
            allowed_grants: vec![],
            access_token_ttl: None,
            refresh_token_ttl: None,
            role_mappings: mappings,
            claims_config: None,
            exchange_audiences: vec![],
            trusted: false,
            legacy_hs256_secret: None,
            disabled: false,
            created_at: 0,
        }
    }

    #[test]
    fn role_mapping_passthrough_and_filter() {
        let roles = vec!["admin".to_string(), "media".to_string()];
        assert_eq!(map_roles(&client(None), &roles), roles);
        let mapped = map_roles(
            &client(Some(json!({"admin": "superuser"}))),
            &roles,
        );
        assert_eq!(mapped, vec!["superuser".to_string()]);
    }
}
