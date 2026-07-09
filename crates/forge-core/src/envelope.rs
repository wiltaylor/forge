//! The Forge response envelope as plain JSON values: `{"ok": true, "data": ...}`
//! on success, `{"ok": false, "error": "..."}` on failure. Transports wrap
//! these with their own status/framing.

use serde::Serialize;
use serde_json::{json, Value};

/// Success envelope with a payload: `{"ok": true, "data": <data>}`.
pub fn ok_value<T: Serialize>(data: T) -> Value {
    json!({ "ok": true, "data": data })
}

/// Success envelope without a payload: `{"ok": true}` (mutations may omit `data`).
pub fn ok_empty_value() -> Value {
    json!({ "ok": true })
}

/// Error envelope: `{"ok": false, "error": "<message>"}`.
pub fn err_value(message: impl Into<String>) -> Value {
    json!({ "ok": false, "error": message.into() })
}
