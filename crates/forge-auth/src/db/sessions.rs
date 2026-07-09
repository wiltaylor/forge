//! Browser sessions and interstitial `/authorize` requests.

use super::models::{AuthRequest, Session};
use super::{opt_row, Db};
use crate::util::now;

impl Db {
    pub async fn session_create(
        &self,
        id_hash: &str,
        user_id: &str,
        amr: &[String],
        idle_ttl: i64,
    ) -> Result<(), sqlx::Error> {
        let ts = now();
        sqlx::query(
            "INSERT INTO sessions (id_hash, user_id, amr, auth_time, created_at, last_seen, expires_at)
             VALUES ($1, $2, $3, $4, $4, $4, $5)",
        )
        .bind(id_hash)
        .bind(user_id)
        .bind(serde_json::to_string(amr).unwrap_or_else(|_| "[]".into()))
        .bind(ts)
        .bind(ts + idle_ttl)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Fetch a live session, bumping `last_seen` and the idle deadline
    /// (capped at the absolute lifetime from `created_at`).
    pub async fn session_touch(
        &self,
        id_hash: &str,
        idle_ttl: i64,
        absolute_ttl: i64,
    ) -> Result<Option<Session>, sqlx::Error> {
        let ts = now();
        let session = opt_row(
            sqlx::query_as::<_, Session>(
                "SELECT * FROM sessions WHERE id_hash = $1 AND revoked_at IS NULL AND expires_at > $2",
            )
            .bind(id_hash)
            .bind(ts)
            .fetch_one(&self.pool)
            .await,
        )?;
        let Some(session) = session else { return Ok(None) };
        if session.created_at + absolute_ttl <= ts {
            return Ok(None);
        }
        let expires = (ts + idle_ttl).min(session.created_at + absolute_ttl);
        sqlx::query("UPDATE sessions SET last_seen = $2, expires_at = $3 WHERE id_hash = $1")
            .bind(id_hash)
            .bind(ts)
            .bind(expires)
            .execute(&self.pool)
            .await?;
        Ok(Some(session))
    }

    pub async fn session_revoke(&self, id_hash: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query(
            "UPDATE sessions SET revoked_at = $2 WHERE id_hash = $1 AND revoked_at IS NULL",
        )
        .bind(id_hash)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn sessions_for_user(&self, user_id: &str) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > $2
             ORDER BY last_seen DESC",
        )
        .bind(user_id)
        .bind(now())
        .fetch_all(&self.pool)
        .await
    }

    pub async fn sessions_list_active(&self) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE revoked_at IS NULL AND expires_at > $1
             ORDER BY last_seen DESC",
        )
        .bind(now())
        .fetch_all(&self.pool)
        .await
    }

    // --- auth requests (interstitial /authorize state) ---

    pub async fn auth_request_create(
        &self,
        id: &str,
        client_id: Option<&str>,
        params: &serde_json::Value,
        ttl: i64,
    ) -> Result<(), sqlx::Error> {
        let ts = now();
        sqlx::query(
            "INSERT INTO auth_requests (id, client_id, params, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(client_id)
        .bind(params.to_string())
        .bind(ts)
        .bind(ts + ttl)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn auth_request_get(&self, id: &str) -> Result<Option<AuthRequest>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, AuthRequest>(
                "SELECT * FROM auth_requests WHERE id = $1 AND expires_at > $2",
            )
            .bind(id)
            .bind(now())
            .fetch_one(&self.pool)
            .await,
        )
    }

    pub async fn auth_request_set_consented(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE auth_requests SET consented = 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn auth_request_set_upstream(
        &self,
        id: &str,
        upstream: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE auth_requests SET upstream = $2 WHERE id = $1")
            .bind(id)
            .bind(upstream.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn auth_request_delete(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM auth_requests WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Periodic cleanup of expired short-lived rows.
    pub async fn sweep_expired(&self) -> Result<(), sqlx::Error> {
        let ts = now();
        sqlx::query("DELETE FROM auth_requests WHERE expires_at <= $1")
            .bind(ts)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM auth_codes WHERE expires_at <= $1")
            .bind(ts)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE expires_at <= $1 OR revoked_at IS NOT NULL AND revoked_at <= $2")
            .bind(ts)
            .bind(ts - 24 * 3600)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM refresh_tokens WHERE expires_at <= $1")
            .bind(ts)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
