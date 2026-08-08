//! HTTP-layer tests for the `/api/data` routes. Doc-store behaviour (name
//! validation on every operation, idempotent delete, atomic writes, list
//! metadata) is tested directly in `forge_core::docstore`; these cover what
//! HTTP adds: routing, the response envelope, status mapping and request-body
//! parsing.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use forge_server::ForgeApp;
use serde_json::json;

fn app(dir: &std::path::Path) -> axum::Router {
    ForgeApp::new("doc-test").with_docstore(dir).router()
}

fn delete(path: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn routes_wrap_the_store_in_envelopes() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());

    let doc = json!({"title": "hello", "items": [1, 2, 3]});
    let (status, body) = send(&router, json_req("PUT", "/api/data/notes", &doc)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok": true}));

    let (status, body) = send(&router, get("/api/data/notes")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok": true, "data": doc}));

    let (status, body) = send(&router, get("/api/data")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["data"][0]["name"], json!("notes"));

    let (status, body) = send(&router, delete("/api/data/notes")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok": true}));
}

// One case per error kind — exhaustive name validation lives in the core tests.

#[tokio::test]
async fn bad_name_maps_to_400() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    let (status, body) = send(&router, get("/api/data/UPPER")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], json!(false));
}

#[tokio::test]
async fn missing_doc_maps_to_404() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    let (status, body) = send(&router, get("/api/data/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));
    assert!(body["error"].as_str().unwrap().contains("nope"));
}

#[tokio::test]
async fn put_invalid_json_400() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    let req = Request::builder()
        .method("PUT")
        .uri("/api/data/notes")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let (status, body) = send(&router, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("not valid JSON"));
}
