//! Crate error type. Every error renders as the Forge error envelope.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::envelope;

/// Errors produced by forge-server. As an [`IntoResponse`] it always emits
/// the `{"ok": false, "error": "..."}` envelope with a matching status.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    /// 400 — malformed input (bad doc name, invalid JSON body, ...).
    #[error("{0}")]
    BadRequest(String),
    /// 401 — missing/invalid/expired credentials.
    #[error("{0}")]
    Unauthorized(String),
    /// 403 — authenticated but not allowed.
    #[error("{0}")]
    Forbidden(String),
    /// 404 — unknown resource/action/route.
    #[error("{0}")]
    NotFound(String),
    /// 500 — configuration problems surfaced at startup.
    #[error("configuration error: {0}")]
    Config(String),
    /// 500 — anything else.
    #[error("{0}")]
    Internal(String),
    /// 500 — I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// 500 — JSON (de)serialization failure outside request parsing.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ForgeError {
    /// HTTP status this error maps to.
    pub fn status(&self) -> StatusCode {
        match self {
            ForgeError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ForgeError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ForgeError::Forbidden(_) => StatusCode::FORBIDDEN,
            ForgeError::NotFound(_) => StatusCode::NOT_FOUND,
            ForgeError::Config(_)
            | ForgeError::Internal(_)
            | ForgeError::Io(_)
            | ForgeError::Json(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ForgeError {
    fn into_response(self) -> Response {
        envelope::err(self.status(), self.to_string())
    }
}
