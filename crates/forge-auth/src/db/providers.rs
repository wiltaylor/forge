//! Upstream identity providers, federated identities and group→role mappings.

use super::models::{GroupMapping, OauthIdentity, UpstreamProvider};
use super::{opt_row, Db};
use crate::error::AppError;
use crate::util::{new_id, now};

impl Db {
    pub async fn providers_list(&self) -> Result<Vec<UpstreamProvider>, sqlx::Error> {
        sqlx::query_as::<_, UpstreamProvider>("SELECT * FROM upstream_providers ORDER BY display_name")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn providers_enabled(&self) -> Result<Vec<UpstreamProvider>, sqlx::Error> {
        sqlx::query_as::<_, UpstreamProvider>(
            "SELECT * FROM upstream_providers WHERE enabled = 1 ORDER BY display_name",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn provider_by_slug(&self, slug: &str) -> Result<Option<UpstreamProvider>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, UpstreamProvider>("SELECT * FROM upstream_providers WHERE slug = $1")
                .bind(slug)
                .fetch_one(&self.pool)
                .await,
        )
    }

    pub async fn provider_by_id(&self, id: &str) -> Result<Option<UpstreamProvider>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, UpstreamProvider>("SELECT * FROM upstream_providers WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn provider_upsert(
        &self,
        slug: &str,
        kind: &str,
        display_name: &str,
        enabled: bool,
        allow_signup: bool,
        link_by_email: bool,
        config: &serde_json::Value,
    ) -> Result<UpstreamProvider, AppError> {
        let ts = now();
        if let Some(existing) = self.provider_by_slug(slug).await? {
            sqlx::query(
                "UPDATE upstream_providers SET kind = $2, display_name = $3, enabled = $4,
                     allow_signup = $5, link_by_email = $6, config = $7, updated_at = $8
                 WHERE slug = $1",
            )
            .bind(slug)
            .bind(kind)
            .bind(display_name)
            .bind(enabled as i64)
            .bind(allow_signup as i64)
            .bind(link_by_email as i64)
            .bind(config.to_string())
            .bind(ts)
            .execute(&self.pool)
            .await?;
            return Ok(self.provider_by_id(&existing.id).await?.expect("just updated"));
        }
        let id = new_id();
        sqlx::query(
            "INSERT INTO upstream_providers (id, slug, kind, display_name, enabled, allow_signup, link_by_email, config, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
        )
        .bind(&id)
        .bind(slug)
        .bind(kind)
        .bind(display_name)
        .bind(enabled as i64)
        .bind(allow_signup as i64)
        .bind(link_by_email as i64)
        .bind(config.to_string())
        .bind(ts)
        .execute(&self.pool)
        .await?;
        Ok(self.provider_by_id(&id).await?.expect("just inserted"))
    }

    pub async fn provider_delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("DELETE FROM upstream_providers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    // --- federated identities ---

    pub async fn identity_lookup(
        &self,
        provider_id: &str,
        subject: &str,
    ) -> Result<Option<OauthIdentity>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, OauthIdentity>(
                "SELECT * FROM oauth_identities WHERE provider_id = $1 AND subject = $2",
            )
            .bind(provider_id)
            .bind(subject)
            .fetch_one(&self.pool)
            .await,
        )
    }

    pub async fn identity_link(
        &self,
        provider_id: &str,
        user_id: &str,
        subject: &str,
        email: Option<&str>,
        raw_claims: Option<&serde_json::Value>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO oauth_identities (id, provider_id, user_id, subject, email, raw_claims, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (provider_id, subject) DO UPDATE SET email = $5, raw_claims = $6",
        )
        .bind(new_id())
        .bind(provider_id)
        .bind(user_id)
        .bind(subject)
        .bind(email)
        .bind(raw_claims.map(|v| v.to_string()))
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn identities_for_user(&self, user_id: &str) -> Result<Vec<OauthIdentity>, sqlx::Error> {
        sqlx::query_as::<_, OauthIdentity>(
            "SELECT * FROM oauth_identities WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn identity_unlink(&self, user_id: &str, identity_id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("DELETE FROM oauth_identities WHERE id = $1 AND user_id = $2")
            .bind(identity_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    // --- group mappings ---

    pub async fn group_mappings_for_provider(
        &self,
        provider_id: &str,
    ) -> Result<Vec<GroupMapping>, sqlx::Error> {
        sqlx::query_as::<_, GroupMapping>(
            "SELECT * FROM group_mappings WHERE provider_id = $1 ORDER BY external_group",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn group_mapping_add(
        &self,
        provider_id: &str,
        external_group: &str,
        role_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO group_mappings (id, provider_id, external_group, role_id) VALUES ($1, $2, $3, $4)
             ON CONFLICT (provider_id, external_group, role_id) DO NOTHING",
        )
        .bind(new_id())
        .bind(provider_id)
        .bind(external_group)
        .bind(role_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn group_mapping_delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("DELETE FROM group_mappings WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn group_mappings_replace(
        &self,
        provider_id: &str,
        mappings: &[(String, String)],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM group_mappings WHERE provider_id = $1")
            .bind(provider_id)
            .execute(&self.pool)
            .await?;
        for (external_group, role_id) in mappings {
            self.group_mapping_add(provider_id, external_group, role_id).await?;
        }
        Ok(())
    }
}
