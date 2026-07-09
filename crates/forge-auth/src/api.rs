//! Forge envelope helper for the `/api/*` surface.

use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

pub fn ok(data: impl Serialize) -> Json<Value> {
    Json(json!({ "ok": true, "data": data }))
}
