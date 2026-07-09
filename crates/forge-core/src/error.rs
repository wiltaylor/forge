//! Crate error type. Transport layers render it as the Forge error envelope
//! with the status from [`ForgeError::status`].

/// Errors produced by Forge backends. Maps to an HTTP-shaped status code via
/// [`ForgeError::status`]; transports build the `{"ok": false, "error": ...}`
/// envelope themselves.
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
    /// HTTP-shaped status code this error maps to.
    pub fn status(&self) -> u16 {
        match self {
            ForgeError::BadRequest(_) => 400,
            ForgeError::Unauthorized(_) => 401,
            ForgeError::Forbidden(_) => 403,
            ForgeError::NotFound(_) => 404,
            ForgeError::Config(_)
            | ForgeError::Internal(_)
            | ForgeError::Io(_)
            | ForgeError::Json(_) => 500,
        }
    }
}
