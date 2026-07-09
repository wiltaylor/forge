use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::Config;
use crate::db::Db;
use crate::tokens::keys::KeySet;

pub struct AppState {
    pub db: Db,
    pub cfg: Config,
    pub keys: RwLock<KeySet>,
    pub http: reqwest::Client,
    pub login_limiter: crate::ratelimit::LoginLimiter,
}

pub type SharedState = Arc<AppState>;
