//! Users, credentials, roles and role assignment.

use sqlx::Row;

use super::models::{Role, User};
use super::{count_from, opt_row, Db};
use crate::error::AppError;
use crate::util::{new_id, now};

pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: Option<&'a str>,
    pub email_verified: bool,
    pub display_name: Option<&'a str>,
}

impl Db {
    pub async fn user_count(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        count_from(row)
    }

    pub async fn user_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await,
        )
    }

    pub async fn user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
                .bind(username)
                .fetch_one(&self.pool)
                .await,
        )
    }

    pub async fn user_by_verified_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, User>(
                "SELECT * FROM users WHERE email = $1 AND email_verified = 1 AND disabled = 0",
            )
            .bind(email)
            .fetch_one(&self.pool)
            .await,
        )
    }

    pub async fn users_list(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn user_create(&self, new: NewUser<'_>) -> Result<User, AppError> {
        if self.user_by_username(new.username).await?.is_some() {
            return Err(AppError::Conflict(format!(
                "username {:?} already exists",
                new.username
            )));
        }
        let id = new_id();
        let ts = now();
        sqlx::query(
            "INSERT INTO users (id, username, email, email_verified, display_name, disabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 0, $6, $6)",
        )
        .bind(&id)
        .bind(new.username)
        .bind(new.email)
        .bind(new.email_verified as i64)
        .bind(new.display_name)
        .bind(ts)
        .execute(&self.pool)
        .await?;
        Ok(self.user_by_id(&id).await?.expect("just inserted"))
    }

    pub async fn user_update(
        &self,
        id: &str,
        email: Option<&str>,
        email_verified: bool,
        display_name: Option<&str>,
        disabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET email = $2, email_verified = $3, display_name = $4, disabled = $5, updated_at = $6
             WHERE id = $1",
        )
        .bind(id)
        .bind(email)
        .bind(email_verified as i64)
        .bind(display_name)
        .bind(disabled as i64)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn user_delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn password_hash_for(&self, user_id: &str) -> Result<Option<String>, sqlx::Error> {
        opt_row(
            sqlx::query("SELECT password_hash FROM credentials WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await,
        )?
        .map(|row| row.try_get::<String, _>("password_hash"))
        .transpose()
    }

    pub async fn password_set(&self, user_id: &str, hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO credentials (user_id, password_hash, updated_at) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET password_hash = $2, updated_at = $3",
        )
        .bind(user_id)
        .bind(hash)
        .bind(now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- roles ---

    pub async fn roles_list(&self) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY name")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn role_by_name(&self, name: &str) -> Result<Option<Role>, sqlx::Error> {
        opt_row(
            sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE name = $1")
                .bind(name)
                .fetch_one(&self.pool)
                .await,
        )
    }

    pub async fn role_create(&self, name: &str, description: Option<&str>) -> Result<Role, AppError> {
        if self.role_by_name(name).await?.is_some() {
            return Err(AppError::Conflict(format!("role {name:?} already exists")));
        }
        let id = new_id();
        sqlx::query("INSERT INTO roles (id, name, description) VALUES ($1, $2, $3)")
            .bind(&id)
            .bind(name)
            .bind(description)
            .execute(&self.pool)
            .await?;
        Ok(Role { id, name: name.into(), description: description.map(Into::into) })
    }

    /// Create-if-missing, used by bootstrap/seeding.
    pub async fn role_ensure(&self, name: &str, description: Option<&str>) -> Result<Role, AppError> {
        if let Some(role) = self.role_by_name(name).await? {
            return Ok(role);
        }
        self.role_create(name, description).await
    }

    pub async fn role_delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        let res = sqlx::query("DELETE FROM roles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn user_role_names(&self, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = $1 ORDER BY r.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| r.try_get("name")).collect()
    }

    pub async fn user_role_add(&self, user_id: &str, role_id: &str, source: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id, source) VALUES ($1, $2, $3)
             ON CONFLICT (user_id, role_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(role_id)
        .bind(source)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Replace a user's roles from `source` with exactly `role_ids`.
    pub async fn user_roles_replace(
        &self,
        user_id: &str,
        role_ids: &[String],
        source: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM user_roles WHERE user_id = $1 AND source = $2")
            .bind(user_id)
            .bind(source)
            .execute(&self.pool)
            .await?;
        for role_id in role_ids {
            self.user_role_add(user_id, role_id, source).await?;
        }
        Ok(())
    }
}
