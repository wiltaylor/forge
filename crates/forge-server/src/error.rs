//! Crate error type (re-exported from forge-core). Renders as the Forge
//! error envelope via [`error_response`].

use axum::http::StatusCode;
use axum::response::Response;

use crate::envelope;

pub use forge_core::ForgeError;

/// Render a [`ForgeError`] as the `{"ok": false, "error": "..."}` envelope
/// with its matching status.
pub(crate) fn error_response(e: ForgeError) -> Response {
    let status = StatusCode::from_u16(e.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    envelope::err(status, e.to_string())
}
