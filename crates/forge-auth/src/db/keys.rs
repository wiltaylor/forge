//! Signing key persistence.

use super::models::SigningKey;
use super::Db;
use crate::util::now;

impl Db {
    pub async fn signing_keys_all(&self) -> Result<Vec<SigningKey>, sqlx::Error> {
        sqlx::query_as::<_, SigningKey>("SELECT * FROM signing_keys ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn signing_key_insert(&self, key: &SigningKey) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO signing_keys (kid, alg, private_pem, public_pem, status, created_at, retired_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&key.kid)
        .bind(&key.alg)
        .bind(&key.private_pem)
        .bind(&key.public_pem)
        .bind(&key.status)
        .bind(key.created_at)
        .bind(key.retired_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Retire every active key (rotation makes a new active one after this).
    pub async fn signing_keys_retire_active(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE signing_keys SET status = 'retired', retired_at = $1 WHERE status = 'active'",
        )
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Drop retired keys older than `grace` seconds (out of JWKS by then).
    pub async fn signing_keys_prune(&self, grace: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM signing_keys WHERE status = 'retired' AND retired_at <= $1")
            .bind(now() - grace)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
