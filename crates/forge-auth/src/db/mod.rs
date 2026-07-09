//! Database access. One [`Db`] wrapping an `AnyPool`, dialect chosen at
//! runtime from `DATABASE_URL` (`sqlite://` or `postgres://`).
//!
//! Query rules for the `Any` driver: `$N` placeholders (valid in both
//! dialects), TEXT/BIGINT/INTEGER-as-bool column types only, no compile-time
//! query macros.

pub mod models;

mod clients;
mod keys;
mod providers;
mod sessions;
mod tokens;
pub mod users;

use sqlx::any::{AnyPoolOptions, AnyRow};
use sqlx::AnyPool;

use crate::error::AppError;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    Sqlite,
    Postgres,
}

#[derive(Clone)]
pub struct Db {
    pub pool: AnyPool,
    pub kind: DbKind,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self, AppError> {
        sqlx::any::install_default_drivers();
        let kind = if database_url.starts_with("sqlite") {
            DbKind::Sqlite
        } else if database_url.starts_with("postgres") {
            DbKind::Postgres
        } else {
            return Err(AppError::Config(format!(
                "DATABASE_URL must start with sqlite:// or postgres://, got {database_url:?}"
            )));
        };

        if kind == DbKind::Sqlite {
            ensure_sqlite_dir(database_url);
        }

        let pool = AnyPoolOptions::new()
            // SQLite writes serialize anyway; a small pool is plenty at this scale.
            .max_connections(if kind == DbKind::Sqlite { 4 } else { 16 })
            .connect(database_url)
            .await?;

        MIGRATOR.run(&pool).await.map_err(AppError::internal)?;
        Ok(Self { pool, kind })
    }
}

/// `sqlite://data/forge-auth.db` fails on a fresh volume if `data/` doesn't
/// exist; create the parent directory up front.
fn ensure_sqlite_dir(database_url: &str) {
    let path = database_url
        .trim_start_matches("sqlite://")
        .split('?')
        .next()
        .unwrap_or_default();
    if path.is_empty() || path == ":memory:" {
        return;
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
}

pub(crate) fn opt_row<T>(res: Result<T, sqlx::Error>) -> Result<Option<T>, sqlx::Error> {
    match res {
        Ok(v) => Ok(Some(v)),
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(e) => Err(e),
    }
}

pub(crate) fn count_from(row: AnyRow) -> Result<i64, sqlx::Error> {
    use sqlx::Row;
    row.try_get::<i64, _>(0)
}
