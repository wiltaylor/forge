//! The Forge response envelope: `{"ok": true, "data": ...}` on success,
//! `{"ok": false, "error": "..."}` with a meaningful HTTP status on failure.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;

/// Success envelope with a payload: `{"ok": true, "data": <data>}`.
pub fn ok<T: Serialize>(data: T) -> Response {
    Json(json!({ "ok": true, "data": data })).into_response()
}

/// Success envelope without a payload: `{"ok": true}` (mutations may omit `data`).
pub fn ok_empty() -> Response {
    Json(json!({ "ok": true })).into_response()
}

/// Error envelope: `{"ok": false, "error": "<message>"}` with the given status.
pub fn err(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "ok": false, "error": message.into() }))).into_response()
}
