mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use common::*;
use forge_server::ForgeApp;
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::json;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tower::ServiceExt;

const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn sse_yields_published_events() {
    let app = ForgeApp::new("sse-test").with_events();
    let bus = app.event_bus();
    let router = app.router();

    let res = router.oneshot(get("/api/events")).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    bus.publish("tick", json!({"n": 1}));

    let mut body = res.into_body();
    let frame = timeout(WAIT, body.frame())
        .await
        .expect("timed out waiting for SSE frame")
        .expect("stream ended")
        .expect("frame error");
    let text = String::from_utf8_lossy(frame.data_ref().unwrap()).into_owned();
    assert!(text.contains("event: tick"), "frame was: {text}");
    assert!(text.contains(r#"data: {"n":1}"#), "frame was: {text}");
}

#[tokio::test]
async fn sse_topics_filter() {
    let app = ForgeApp::new("sse-test").with_events();
    let bus = app.event_bus();
    let router = app.router();

    let res = router
        .oneshot(get("/api/events?topics=wanted,also"))
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    bus.publish("ignored", json!({"drop": true}));
    bus.publish("wanted", json!({"keep": true}));

    let mut body = res.into_body();
    let frame = timeout(WAIT, body.frame())
        .await
        .expect("timed out waiting for SSE frame")
        .unwrap()
        .unwrap();
    let text = String::from_utf8_lossy(frame.data_ref().unwrap()).into_owned();
    assert!(
        text.contains("event: wanted") && !text.contains("ignored"),
        "first delivered frame should be the filtered topic, got: {text}"
    );
}

#[tokio::test]
async fn sse_requires_token_when_auth_enabled() {
    let cfg = forge_server::AuthConfig::new("0123456789abcdef0123456789abcdef").user("a", "b");
    let router = ForgeApp::new("t").auth(cfg).with_events().router();
    let (status, body) = send(&router, get("/api/events")).await;
    assert_eq!(status, 401);
    assert_eq!(body["ok"], json!(false));
}

async fn ws_recv_json(
    ws: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> serde_json::Value {
    loop {
        let msg = timeout(WAIT, ws.next())
            .await
            .expect("timed out waiting for ws frame")
            .expect("ws closed")
            .expect("ws error");
        if let Message::Text(text) = msg {
            return serde_json::from_str(&text).unwrap();
        }
    }
}

/// Serve the router over an in-memory duplex pipe and complete the WebSocket
/// handshake against it — no TCP listener involved.
async fn ws_connect(
    router: axum::Router,
) -> tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream> {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        hyper::server::conn::http1::Builder::new()
            .serve_connection(
                hyper_util::rt::TokioIo::new(server_io),
                hyper_util::service::TowerToHyperService::new(router),
            )
            .with_upgrades()
            .await
            .ok();
    });
    let (ws, _) = tokio_tungstenite::client_async("ws://forge.test/api/ws", client_io)
        .await
        .expect("ws handshake");
    ws
}

#[tokio::test]
async fn ws_subscribe_ping_and_events() {
    let app = ForgeApp::new("ws-test").with_events();
    let bus = app.event_bus();
    let router = app.router();

    let mut ws = ws_connect(router).await;

    // Filter to topic "a", then ping — the pong confirms the subscribe was
    // processed (frames are handled in order).
    ws.send(Message::text(r#"{"type":"subscribe","topics":["a"]}"#))
        .await
        .unwrap();
    ws.send(Message::text(r#"{"type":"ping"}"#)).await.unwrap();
    let pong = ws_recv_json(&mut ws).await;
    assert_eq!(pong, json!({"type": "pong"}));

    bus.publish("b", json!({"x": 1})); // filtered out
    bus.publish("a", json!({"x": 2}));

    let event = ws_recv_json(&mut ws).await;
    assert_eq!(
        event,
        json!({"type": "event", "topic": "a", "data": {"x": 2}})
    );

    ws.close(None).await.ok();
}

#[tokio::test]
async fn ws_default_subscription_is_all_topics() {
    let app = ForgeApp::new("ws-test").with_events();
    let bus = app.event_bus();
    let router = app.router();

    let mut ws = ws_connect(router).await;
    // Ping/pong round-trip guarantees the server task (and its bus
    // subscription) is live before we publish.
    ws.send(Message::text(r#"{"type":"ping"}"#)).await.unwrap();
    assert_eq!(ws_recv_json(&mut ws).await, json!({"type": "pong"}));

    bus.publish("anything", json!("payload"));
    let event = ws_recv_json(&mut ws).await;
    assert_eq!(
        event,
        json!({"type": "event", "topic": "anything", "data": "payload"})
    );
    ws.close(None).await.ok();
}

#[tokio::test]
async fn ws_rejected_without_token_when_auth_enabled() {
    let cfg = forge_server::AuthConfig::new("0123456789abcdef0123456789abcdef").user("a", "b");
    let router = ForgeApp::new("t").auth(cfg).with_events().router();
    let req = Request::builder()
        .uri("/api/ws")
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&router, req).await;
    assert_eq!(status, 401);
    assert_eq!(body["ok"], json!(false));
}
