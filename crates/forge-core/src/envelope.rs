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

#[cfg(test)]
mod tests {
    use super::{err_value, ok_empty_value, ok_value};
    use serde_json::json;

    #[test]
    fn success_envelope_wraps_the_data() {
        assert_eq!(ok_value(json!([1, 2])), json!({"ok": true, "data": [1, 2]}));
        // `null` data is still a `data` key, not an omission.
        assert_eq!(
            ok_value(serde_json::Value::Null),
            json!({"ok": true, "data": null})
        );
    }

    #[test]
    fn empty_success_envelope_has_no_data_key() {
        assert_eq!(ok_empty_value(), json!({"ok": true}));
    }

    #[test]
    fn error_envelope_carries_the_message() {
        assert_eq!(err_value("boom"), json!({"ok": false, "error": "boom"}));
    }
}
