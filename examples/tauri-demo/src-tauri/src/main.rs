//! Forge tauri-demo — the whole backend is this file: a Forge plugin with a
//! doc store, two actions, all three widgets, and a background ticker.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};

fn main() {
    let forge = forge_tauri::Builder::new("tauri-demo")
        .with_docstore_default()
        .with_term()
        .with_vnc()
        .with_rdp()
        .action("system_info", |_payload, _ctx| async move {
            Ok(json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "pid": std::process::id(),
            }))
        })
        .action("publish", |payload: Value, ctx| async move {
            let topic = payload
                .get("topic")
                .and_then(Value::as_str)
                .unwrap_or("misc")
                .to_string();
            let data = payload.get("data").cloned().unwrap_or(Value::Null);
            ctx.events.publish(&topic, data);
            Ok(json!({ "published": true, "topic": topic }))
        });

    // Background ticker so the Overview event log has something live.
    let bus = forge.event_bus();
    tauri::async_runtime::spawn(async move {
        let mut n: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            n += 1;
            bus.publish("ticks", json!({ "n": n, "source": "tauri-demo" }));
        }
    });

    tauri::Builder::default()
        .plugin(forge.build())
        .run(tauri::generate_context!())
        .expect("error while running tauri-demo");
}
