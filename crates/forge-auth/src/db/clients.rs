//! OIDC client (relying party) persistence.

use super::models::Client;
use super::{opt_row, Db};
use crate::error::AppError;
use crate::util::now;

fn to_json(list: &[String]) -> String {
    serde_json::to_string(list).unwrap_or_else(|_| "[]".into())
}

impl Db {
    pub async fn client_by_id(&self, id: &str) -> Result<Option<Client>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, Client>("SELECT * FROM clients WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await,
        )
    }

    pub async fn clients_list(&self) -> Result<Vec<Client>, sqlx::Error> {
        sqlx::query_as::<_, Client>("SELECT * FROM clients ORDER BY name")
            .fetch_all(&self.pool)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn client_create(&self, client: &Client) -> Result<(), AppError> {
        if self.client_by_id(&client.id).await?.is_some() {
            return Err(AppError::Conflict(format!(
                "client {:?} already exists",
                client.id
            )));
        }
        sqlx::query(
            "INSERT INTO clients (id, name, client_type, secret_hash, redirect_uris, post_logout_redirect_uris,
                 allowed_scopes, allowed_grants, client_roles, access_token_ttl, refresh_token_ttl, role_mappings,
                 claims_config, exchange_audiences, trusted, legacy_hs256_secret, disabled, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
        )
        .bind(&client.id)
        .bind(&client.name)
        .bind(&client.client_type)
        .bind(&client.secret_hash)
        .bind(to_json(&client.redirect_uris))
        .bind(to_json(&client.post_logout_redirect_uris))
        .bind(to_json(&client.allowed_scopes))
        .bind(to_json(&client.allowed_grants))
        .bind(to_json(&client.client_roles))
        .bind(client.access_token_ttl)
        .bind(client.refresh_token_ttl)
        .bind(client.role_mappings.as_ref().map(|v| v.to_string()))
        .bind(client.claims_config.as_ref().map(|v| v.to_string()))
        .bind(to_json(&client.exchange_audiences))
        .bind(client.trusted as i64)
        .bind(&client.legacy_hs256_secret)
        .bind(client.disabled as i64)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn client_update(&self, client: &Client) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE clients SET name = $2, client_type = $3, redirect_uris = $4, post_logout_redirect_uris = $5,
                 allowed_scopes = $6, allowed_grants = $7, client_roles = $8, access_token_ttl = $9, refresh_token_ttl = $10,
                 role_mappings = $11, claims_config = $12, exchange_audiences = $13, trusted = $14,
                 legacy_hs256_secret = $15, disabled = $16
             WHERE id = $1",
        )
        .bind(&client.id)
        .bind(&client.name)
        .bind(&client.client_type)
        .bind(to_json(&client.redirect_uris))
        .bind(to_json(&client.post_logout_redirect_uris))
        .bind(to_json(&client.allowed_scopes))
        .bind(to_json(&client.allowed_grants))
        .bind(to_json(&client.client_roles))
        .bind(client.access_token_ttl)
        .bind(client.refresh_token_ttl)
        .bind(client.role_mappings.as_ref().map(|v| v.to_string()))
        .bind(client.claims_config.as_ref().map(|v| v.to_string()))
        .bind(to_json(&client.exchange_audiences))
        .bind(client.trusted as i64)
        .bind(&client.legacy_hs256_secret)
        .bind(client.disabled as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn client_set_secret(&self, id: &str, secret_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE clients SET secret_hash = $2 WHERE id = $1")
            .bind(id)
            .bind(secret_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn client_delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("DELETE FROM clients WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}
