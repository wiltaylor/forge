//! Two error surfaces: [`AppError`] renders the forge `{"ok":false,...}`
//! envelope for `/api/*` routes; [`OAuthError`] renders RFC 6749 error JSON
//! for `/oauth2/*` routes.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::Internal(e.to_string())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Config(_) | Self::Db(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        // Never leak internals to the client; the log has the detail.
        let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %self, "internal error");
            "internal error".to_string()
        } else {
            self.to_string()
        };
        (status, Json(json!({ "ok": false, "error": message }))).into_response()
    }
}

/// RFC 6749 §5.2-shaped error for the OAuth/OIDC protocol surface.
#[derive(Debug)]
pub struct OAuthError {
    pub status: StatusCode,
    pub error: &'static str,
    pub description: String,
}

impl OAuthError {
    pub fn invalid_request(description: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_request",
            description: description.into(),
        }
    }
    pub fn invalid_client(description: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            error: "invalid_client",
            description: description.into(),
        }
    }
    pub fn invalid_grant(description: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_grant",
            description: description.into(),
        }
    }
    pub fn unsupported_grant_type(description: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "unsupported_grant_type",
            description: description.into(),
        }
    }
    pub fn invalid_scope(description: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_scope",
            description: description.into(),
        }
    }
    pub fn invalid_target(description: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "invalid_target",
            description: description.into(),
        }
    }
    pub fn server_error(e: impl std::fmt::Display) -> Self {
        tracing::error!(error = %e, "oauth server error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "server_error",
            description: "internal error".into(),
        }
    }
}

impl From<AppError> for OAuthError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Unauthorized => Self::invalid_client("client authentication failed"),
            AppError::NotFound | AppError::BadRequest(_) => Self::invalid_request(e.to_string()),
            other => Self::server_error(other),
        }
    }
}

impl From<sqlx::Error> for OAuthError {
    fn from(e: sqlx::Error) -> Self {
        Self::server_error(e)
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let mut res = (
            self.status,
            Json(json!({ "error": self.error, "error_description": self.description })),
        )
            .into_response();
        if self.error == "invalid_client" {
            res.headers_mut().insert(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Basic realm=\"forge-auth\""),
            );
        }
        res
    }
}
