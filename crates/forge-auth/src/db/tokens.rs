//! Authorization codes and refresh tokens (rotation with reuse detection).

use super::models::{AuthCode, RefreshToken};
use super::{opt_row, Db};
use crate::util::{new_id, now};

impl Db {
    pub async fn auth_code_insert(&self, code: &AuthCode) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO auth_codes (code_hash, client_id, user_id, redirect_uri, scope, nonce,
                 code_challenge, code_challenge_method, auth_time, amr, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&code.code_hash)
        .bind(&code.client_id)
        .bind(&code.user_id)
        .bind(&code.redirect_uri)
        .bind(&code.scope)
        .bind(&code.nonce)
        .bind(&code.code_challenge)
        .bind(&code.code_challenge_method)
        .bind(code.auth_time)
        .bind(serde_json::to_string(&code.amr).unwrap_or_else(|_| "[]".into()))
        .bind(code.created_at)
        .bind(code.expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Atomically consume a code: the UPDATE only wins once, so a replayed
    /// code returns `None` even under concurrent requests.
    pub async fn auth_code_consume(&self, code_hash: &str) -> Result<Option<AuthCode>, sqlx::Error> {
        let ts = now();
        let res = sqlx::query(
            "UPDATE auth_codes SET consumed_at = $2
             WHERE code_hash = $1 AND consumed_at IS NULL AND expires_at > $2",
        )
        .bind(code_hash)
        .bind(ts)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Ok(None);
        }
        opt_row(
            sqlx::query_as::<_, AuthCode>("SELECT * FROM auth_codes WHERE code_hash = $1")
                .bind(code_hash)
                .fetch_one(&self.pool)
                .await,
        )
    }

    // --- refresh tokens ---

    pub async fn refresh_token_insert(
        &self,
        token_hash: &str,
        family_id: &str,
        user_id: &str,
        client_id: &str,
        scope: &str,
        parent_id: Option<&str>,
        ttl: i64,
    ) -> Result<String, sqlx::Error> {
        let id = new_id();
        let ts = now();
        sqlx::query(
            "INSERT INTO refresh_tokens (id, family_id, user_id, client_id, token_hash, scope, parent_id, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&id)
        .bind(family_id)
        .bind(user_id)
        .bind(client_id)
        .bind(token_hash)
        .bind(scope)
        .bind(parent_id)
        .bind(ts)
        .bind(ts + ttl)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn refresh_token_by_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, RefreshToken>("SELECT * FROM refresh_tokens WHERE token_hash = $1")
                .bind(token_hash)
                .fetch_one(&self.pool)
                .await,
        )
    }

    /// Atomically mark a token used; returns false when it was already used
    /// (reuse — the caller must revoke the family).
    pub async fn refresh_token_mark_used(&self, id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query(
            "UPDATE refresh_tokens SET used_at = $2 WHERE id = $1 AND used_at IS NULL AND revoked_at IS NULL",
        )
        .bind(id)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn refresh_family_revoke(&self, family_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = $2 WHERE family_id = $1 AND revoked_at IS NULL")
            .bind(family_id)
            .bind(now())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn refresh_tokens_revoke_for_user(&self, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = $2 WHERE user_id = $1 AND revoked_at IS NULL")
            .bind(user_id)
            .bind(now())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
