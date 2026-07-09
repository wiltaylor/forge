//! /api/desktop/vnc session behaviour: config gating, connect failures, and
//! a live round-trip behind #[ignore]. Route mounting + auth follow the same
//! wiring as /api/term (widgets_ws.rs), so these routers run auth-disabled.
#![cfg(feature = "vnc")]

use std::time::Duration;

use forge_server::widgets::DesktopConfig;
use forge_server::ForgeApp;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Ws = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

const WAIT: Duration = Duration::from_secs(15);

async fn spawn(router: axum::Router) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

async fn connect_vnc_ws(config: DesktopConfig) -> Ws {
    let router = ForgeApp::new("t").with_vnc_config(config).router();
    let addr = spawn(router).await;
    let (ws, _) = connect_async(format!("ws://{addr}/api/desktop/vnc"))
        .await
        .expect("ws connect");
    ws
}

async fn send_json(ws: &mut Ws, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

/// Read frames until a text control frame arrives, returning it parsed.
async fn next_ctrl(ws: &mut Ws) -> Value {
    loop {
        let msg = timeout(WAIT, ws.next())
            .await
            .expect("timed out waiting for ws frame")
            .expect("ws closed")
            .expect("ws error");
        match msg {
            Message::Text(t) => return serde_json::from_str(&t).unwrap(),
            _ => continue,
        }
    }
}

#[tokio::test]
async fn missing_host_yields_error() {
    let mut ws = connect_vnc_ws(DesktopConfig::default()).await;
    send_json(&mut ws, json!({"type":"connect"})).await;
    let msg = next_ctrl(&mut ws).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("host"));
}

#[tokio::test]
async fn host_not_in_allowlist_yields_error() {
    let mut ws = connect_vnc_ws(DesktopConfig {
        allow_hosts: Some(vec!["allowed.example".into()]),
    })
    .await;
    send_json(
        &mut ws,
        json!({"type":"connect","host":"other.example","port":5900}),
    )
    .await;
    let msg = next_ctrl(&mut ws).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("allowed hosts"));
}

#[tokio::test]
async fn refused_target_yields_error() {
    let mut ws = connect_vnc_ws(DesktopConfig::default()).await;
    // Port 9 (discard) on localhost: nothing listens in the test env.
    send_json(
        &mut ws,
        json!({"type":"connect","host":"127.0.0.1","port":9}),
    )
    .await;
    let msg = next_ctrl(&mut ws).await;
    assert_eq!(msg["type"], "error");
}

#[tokio::test]
async fn non_connect_first_frame_yields_error() {
    let mut ws = connect_vnc_ws(DesktopConfig::default()).await;
    send_json(&mut ws, json!({"type":"mouse","x":1,"y":1,"buttons":0})).await;
    let msg = next_ctrl(&mut ws).await;
    assert_eq!(msg["type"], "error");
    assert!(msg["message"].as_str().unwrap().contains("connect"));
}

/// Live VNC round-trip against a real server. Run with:
/// `FORGE_TEST_VNC_ADDR=host:port FORGE_TEST_VNC_PASS=p \
///  cargo test -p forge-server --features vnc -- --ignored vnc_live`
#[tokio::test]
#[ignore = "needs FORGE_TEST_VNC_ADDR (+ optional FORGE_TEST_VNC_PASS)"]
async fn vnc_live_frames_round_trip() {
    let addr = std::env::var("FORGE_TEST_VNC_ADDR").expect("FORGE_TEST_VNC_ADDR");
    let pass = std::env::var("FORGE_TEST_VNC_PASS").unwrap_or_default();
    let (host, port) = match addr.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().expect("port")),
        None => (addr, 5900),
    };

    let mut ws = connect_vnc_ws(DesktopConfig::default()).await;
    send_json(
        &mut ws,
        json!({"type":"connect","host":host,"port":port,"password":pass}),
    )
    .await;

    let ready = next_ctrl(&mut ws).await;
    assert_eq!(ready["type"], "ready", "vnc connect failed: {ready}");
    let width = ready["width"].as_u64().unwrap();
    let height = ready["height"].as_u64().unwrap();
    assert!(width > 0 && height > 0);

    // The initial full framebuffer update must arrive as valid rect frames.
    let deadline = tokio::time::Instant::now() + WAIT;
    let mut rects = 0u32;
    while rects == 0 {
        let msg = tokio::time::timeout_at(deadline, ws.next())
            .await
            .expect("timed out waiting for a rect frame")
            .expect("ws closed")
            .expect("ws error");
        if let Message::Binary(b) = msg {
            assert!(b.len() >= 10, "runt rect frame");
            assert_eq!(b[0], 1, "rect version");
            assert_eq!(b[1], 0, "raw encoding");
            let w = u16::from_le_bytes([b[6], b[7]]) as usize;
            let h = u16::from_le_bytes([b[8], b[9]]) as usize;
            assert_eq!(b.len(), 10 + w * h * 4, "payload length");
            rects += 1;
        }
    }

    // Exercise the input path (no assertion beyond "connection survives").
    send_json(&mut ws, json!({"type":"mouse","x":10,"y":10,"buttons":0})).await;
    send_json(
        &mut ws,
        json!({"type":"key","code":"KeyA","key":"a","down":true}),
    )
    .await;
    send_json(
        &mut ws,
        json!({"type":"key","code":"KeyA","key":"a","down":false}),
    )
    .await;
    send_json(&mut ws, json!({"type":"wheel","dx":0,"dy":-120})).await;
    ws.close(None).await.unwrap();
}
