//! Forge rust-demo — serves the gallery frontend from inside the binary.
//!
//! Debug builds read `../../apps/gallery/dist` from disk (edit + refresh);
//! `cargo build --release -p rust-demo` embeds it: one self-contained binary
//! that only needs a `.env` next to it.

use forge_server::{ForgeApp, ForgeError};
use serde_json::{json, Value};

#[derive(rust_embed::RustEmbed)]
#[folder = "../../apps/gallery/dist"]
struct Assets;

#[tokio::main]
async fn main() -> Result<(), ForgeError> {
    let mut app = ForgeApp::new("rust-demo")
        .with_docstore_from_env()
        .with_events()
        .with_components_from_env()
        // Remote-access widgets: off unless FORGE_TERM_ENABLE / FORGE_VNC_ENABLE /
        // FORGE_RDP_ENABLE are set (see docs/widgets-protocol.md).
        .with_term_from_env()
        .with_vnc_from_env()
        .with_rdp_from_env()
        .action("echo", |payload, _ctx| async move { Ok(payload) })
        .action("publish", |payload: Value, ctx| async move {
            let topic = payload
                .get("topic")
                .and_then(Value::as_str)
                .unwrap_or("misc")
                .to_string();
            let data = payload.get("data").cloned().unwrap_or(Value::Null);
            ctx.events.publish(&topic, data);
            Ok(json!({ "published": true, "topic": topic }))
        })
        .frontend_embedded::<Assets>();

    // Auth is on when the .env provides a secret; without one the demo runs open.
    if std::env::var("FORGE_JWT_SECRET").is_ok() {
        app = app.auth_from_env();
    }

    // Background ticker so the gallery's Live section has something to show.
    let bus = app.event_bus();
    tokio::spawn(async move {
        let mut n: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            n += 1;
            bus.publish("ticks", json!({ "n": n, "source": "rust-demo" }));
        }
    });

    // Clickable URL (same FORGE_HOST/FORGE_PORT resolution as serve(); the
    // .env is already loaded by ForgeApp::new above).
    let host = std::env::var("FORGE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("FORGE_PORT").unwrap_or_else(|_| "8765".into());
    let display_host = if host == "0.0.0.0" { "127.0.0.1" } else { &host };
    println!("\n  rust-demo → http://{display_host}:{port}/\n");

    app.serve().await
}
