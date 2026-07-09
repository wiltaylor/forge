//! Widget WebSocket wiring: routes mount only when enabled, sit behind the
//! auth middleware, and upgrade successfully. Session behaviour is covered by
//! the per-widget tests (term_ws.rs, ...).
#![cfg(feature = "term")]

mod common;

use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use common::*;
use forge_server::ForgeApp;
use futures_util::StreamExt;
use serde_json::json;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

const WAIT: Duration = Duration::from_secs(5);
const SECRET: &str = "0123456789abcdef0123456789abcdef";

fn ws_get(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap()
}

async fn spawn(router: axum::Router) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    addr
}

#[tokio::test]
async fn term_route_absent_without_builder() {
    // Feature compiled but with_term() not called → no route.
    let router = ForgeApp::new("t").router();
    let (status, body) = send(&router, ws_get("/api/term")).await;
    assert_eq!(status, 404);
    assert_eq!(body["ok"], json!(false));
}

#[tokio::test]
async fn term_rejected_without_token_when_auth_enabled() {
    let cfg = forge_server::AuthConfig::new(SECRET).user("a", "b");
    let router = ForgeApp::new("t").auth(cfg).with_term().router();
    let (status, body) = send(&router, ws_get("/api/term")).await;
    assert_eq!(status, 401);
    assert_eq!(body["ok"], json!(false));
}

#[tokio::test]
async fn term_upgrades_with_query_token() {
    let cfg = forge_server::AuthConfig::new(SECRET).user("admin", "hunter2");
    let router = ForgeApp::new("t").auth(cfg).with_term().router();

    let (status, body) = send(
        &router,
        json_req(
            "POST",
            "/api/auth/login",
            &json!({"username": "admin", "password": "hunter2"}),
        ),
    )
    .await;
    assert_eq!(status, 200);
    let token = body["data"]["token"].as_str().unwrap().to_string();

    let addr = spawn(router).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/api/term?token={token}"))
        .await
        .expect("ws connect with ?token=");

    // The stub session answers with an error control frame — receiving it
    // proves auth passed and the upgrade completed.
    let msg = timeout(WAIT, ws.next())
        .await
        .expect("timed out waiting for ws frame")
        .expect("ws closed")
        .expect("ws error");
    let Message::Text(text) = msg else {
        panic!("expected text control frame, got: {msg:?}")
    };
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["type"], json!("error"));
}
