mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use forge_server::{ForgeApp, ForgeError};
use serde_json::json;

fn app() -> axum::Router {
    ForgeApp::new("action-test")
        .action("echo", |payload, _ctx| async move { Ok(payload) })
        .action("greet", |payload, ctx| async move {
            let who = payload["who"].as_str().unwrap_or("world").to_string();
            ctx.events.publish("greeted", &who);
            Ok(json!({ "greeting": format!("hello {who}"), "as": ctx.claims.sub }))
        })
        .action("boom", |_payload, _ctx| async move {
            Err::<serde_json::Value, _>(ForgeError::BadRequest("boom".into()))
        })
        .router()
}

#[tokio::test]
async fn echo_roundtrip() {
    let router = app();
    let payload = json!({"x": 1, "y": ["a", "b"]});
    let (status, body) = send(&router, json_req("POST", "/api/actions/echo", &payload)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["data"], payload);
}

#[tokio::test]
async fn empty_body_is_empty_object() {
    let router = app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/actions/echo")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&router, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!({}));
}

#[tokio::test]
async fn ctx_has_claims_and_events() {
    let router = app();
    let (status, body) = send(
        &router,
        json_req("POST", "/api/actions/greet", &json!({"who": "forge"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["greeting"], json!("hello forge"));
    // Auth disabled → anonymous identity in the action context.
    assert_eq!(body["data"]["as"], json!("anonymous"));
}

#[tokio::test]
async fn action_error_maps_to_envelope() {
    let router = app();
    let (status, body) = send(&router, json_req("POST", "/api/actions/boom", &json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!({"ok": false, "error": "boom"}));
}

#[tokio::test]
async fn unknown_action_404_lists_registered() {
    let router = app();
    let (status, body) = send(&router, json_req("POST", "/api/actions/nope", &json!({}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));
    let msg = body["error"].as_str().unwrap();
    assert!(msg.contains("nope"), "names the unknown action: {msg}");
    for name in ["boom", "echo", "greet"] {
        assert!(msg.contains(name), "lists registered action {name}: {msg}");
    }
}

#[tokio::test]
async fn invalid_json_body_400() {
    let router = app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/actions/echo")
        .body(Body::from("{oops"))
        .unwrap();
    let (status, body) = send(&router, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("not valid JSON"));
}

#[tokio::test]
async fn health_lists_actions_sorted() {
    let router = app();
    let (status, body) = send(&router, get("/api/health")).await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];
    assert_eq!(data["app"], json!("action-test"));
    assert_eq!(data["actions"], json!(["boom", "echo", "greet"]));
    assert_eq!(data["version"], json!(env!("CARGO_PKG_VERSION")));
    assert!(data["uptime_s"].as_f64().unwrap() >= 0.0);
}
