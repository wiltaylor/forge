//! The health payload shape shared by every transport
//! (`GET /api/health` over HTTP, the `request` command over Tauri IPC).

use serde_json::{json, Value};

/// Build the health report. `uptime_s` is rounded to one decimal here so all
/// transports agree on the wire shape.
pub fn health_payload(
    app: &str,
    uptime_s: f64,
    version: &str,
    auth_enabled: bool,
    actions: &[&str],
) -> Value {
    let uptime = (uptime_s * 10.0).round() / 10.0;
    json!({
        "uptime_s": uptime,
        "version": version,
        "app": app,
        "auth_enabled": auth_enabled,
        "actions": actions,
    })
}
