//! Row types. Schema rule: TEXT / BIGINT / INTEGER-as-bool only (sqlx `Any`
//! driver common subset), JSON stored as TEXT.

use serde::Serialize;
use sqlx::any::AnyRow;
use sqlx::Row;

fn get_bool(row: &AnyRow, col: &str) -> sqlx::Result<bool> {
    Ok(row.try_get::<i64, _>(col)? != 0)
}

fn get_json_vec(row: &AnyRow, col: &str) -> sqlx::Result<Vec<String>> {
    let raw: String = row.try_get(col)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub disabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for User {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            email: row.try_get("email")?,
            email_verified: get_bool(row, "email_verified")?,
            display_name: row.try_get("display_name")?,
            disabled: get_bool(row, "disabled")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for Role {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SigningKey {
    pub kid: String,
    pub alg: String,
    pub private_pem: String,
    pub public_pem: String,
    pub status: String,
    pub created_at: i64,
    pub retired_at: Option<i64>,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for SigningKey {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            kid: row.try_get("kid")?,
            alg: row.try_get("alg")?,
            private_pem: row.try_get("private_pem")?,
            public_pem: row.try_get("public_pem")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            retired_at: row.try_get("retired_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id_hash: String,
    pub user_id: String,
    pub amr: Vec<String>,
    pub auth_time: i64,
    pub created_at: i64,
    pub last_seen: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for Session {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            id_hash: row.try_get("id_hash")?,
            user_id: row.try_get("user_id")?,
            amr: get_json_vec(row, "amr")?,
            auth_time: row.try_get("auth_time")?,
            created_at: row.try_get("created_at")?,
            last_seen: row.try_get("last_seen")?,
            expires_at: row.try_get("expires_at")?,
            revoked_at: row.try_get("revoked_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Client {
    pub id: String,
    pub name: String,
    pub client_type: String,
    #[serde(skip)]
    pub secret_hash: Option<String>,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub allowed_grants: Vec<String>,
    pub access_token_ttl: Option<i64>,
    pub refresh_token_ttl: Option<i64>,
    /// `{"idp role name": "emitted role name"}`; `None` = pass roles through.
    pub role_mappings: Option<serde_json::Value>,
    pub claims_config: Option<serde_json::Value>,
    pub exchange_audiences: Vec<String>,
    pub trusted: bool,
    #[serde(skip)]
    pub legacy_hs256_secret: Option<String>,
    pub disabled: bool,
    pub created_at: i64,
}

impl Client {
    pub fn has_legacy_secret(&self) -> bool {
        self.legacy_hs256_secret.is_some()
    }
}

impl<'r> sqlx::FromRow<'r, AnyRow> for Client {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        let role_mappings: Option<String> = row.try_get("role_mappings")?;
        let claims_config: Option<String> = row.try_get("claims_config")?;
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            client_type: row.try_get("client_type")?,
            secret_hash: row.try_get("secret_hash")?,
            redirect_uris: get_json_vec(row, "redirect_uris")?,
            post_logout_redirect_uris: get_json_vec(row, "post_logout_redirect_uris")?,
            allowed_scopes: get_json_vec(row, "allowed_scopes")?,
            allowed_grants: get_json_vec(row, "allowed_grants")?,
            access_token_ttl: row.try_get("access_token_ttl")?,
            refresh_token_ttl: row.try_get("refresh_token_ttl")?,
            role_mappings: role_mappings.and_then(|s| serde_json::from_str(&s).ok()),
            claims_config: claims_config.and_then(|s| serde_json::from_str(&s).ok()),
            exchange_audiences: get_json_vec(row, "exchange_audiences")?,
            trusted: get_bool(row, "trusted")?,
            legacy_hs256_secret: row.try_get("legacy_hs256_secret")?,
            disabled: get_bool(row, "disabled")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub id: String,
    pub client_id: Option<String>,
    pub params: serde_json::Value,
    pub consented: bool,
    pub upstream: Option<serde_json::Value>,
    pub created_at: i64,
    pub expires_at: i64,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for AuthRequest {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        let params: String = row.try_get("params")?;
        let upstream: Option<String> = row.try_get("upstream")?;
        Ok(Self {
            id: row.try_get("id")?,
            client_id: row.try_get("client_id")?,
            params: serde_json::from_str(&params).unwrap_or_default(),
            consented: get_bool(row, "consented")?,
            upstream: upstream.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthCode {
    pub code_hash: String,
    pub client_id: String,
    pub user_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub nonce: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub auth_time: i64,
    pub amr: Vec<String>,
    pub created_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for AuthCode {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            code_hash: row.try_get("code_hash")?,
            client_id: row.try_get("client_id")?,
            user_id: row.try_get("user_id")?,
            redirect_uri: row.try_get("redirect_uri")?,
            scope: row.try_get("scope")?,
            nonce: row.try_get("nonce")?,
            code_challenge: row.try_get("code_challenge")?,
            code_challenge_method: row.try_get("code_challenge_method")?,
            auth_time: row.try_get("auth_time")?,
            amr: get_json_vec(row, "amr")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
            consumed_at: row.try_get("consumed_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: String,
    pub family_id: String,
    pub user_id: String,
    pub client_id: String,
    pub token_hash: String,
    pub scope: String,
    pub parent_id: Option<String>,
    pub created_at: i64,
    pub used_at: Option<i64>,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for RefreshToken {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            family_id: row.try_get("family_id")?,
            user_id: row.try_get("user_id")?,
            client_id: row.try_get("client_id")?,
            token_hash: row.try_get("token_hash")?,
            scope: row.try_get("scope")?,
            parent_id: row.try_get("parent_id")?,
            created_at: row.try_get("created_at")?,
            used_at: row.try_get("used_at")?,
            expires_at: row.try_get("expires_at")?,
            revoked_at: row.try_get("revoked_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UpstreamProvider {
    pub id: String,
    pub slug: String,
    pub kind: String,
    pub display_name: String,
    pub enabled: bool,
    pub allow_signup: bool,
    pub link_by_email: bool,
    pub config: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for UpstreamProvider {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        let config: String = row.try_get("config")?;
        Ok(Self {
            id: row.try_get("id")?,
            slug: row.try_get("slug")?,
            kind: row.try_get("kind")?,
            display_name: row.try_get("display_name")?,
            enabled: get_bool(row, "enabled")?,
            allow_signup: get_bool(row, "allow_signup")?,
            link_by_email: get_bool(row, "link_by_email")?,
            config: serde_json::from_str(&config).unwrap_or_default(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OauthIdentity {
    pub id: String,
    pub provider_id: String,
    pub user_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub created_at: i64,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for OauthIdentity {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            provider_id: row.try_get("provider_id")?,
            user_id: row.try_get("user_id")?,
            subject: row.try_get("subject")?,
            email: row.try_get("email")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupMapping {
    pub id: String,
    pub provider_id: String,
    pub external_group: String,
    pub role_id: String,
}

impl<'r> sqlx::FromRow<'r, AnyRow> for GroupMapping {
    fn from_row(row: &'r AnyRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            provider_id: row.try_get("provider_id")?,
            external_group: row.try_get("external_group")?,
            role_id: row.try_get("role_id")?,
        })
    }
}
