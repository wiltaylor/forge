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
async fn bad_names_400() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    for name in ["UPPER", "_lead", "-lead", "has.dot"] {
        let (status, body) = send(&router, get(&format!("/api/data/{name}"))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "GET {name}");
        assert_eq!(body["ok"], json!(false));
        let (status, _) = send(
            &router,
            json_req("PUT", &format!("/api/data/{name}"), &json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "PUT {name}");
        let (status, _) = send(&router, delete(&format!("/api/data/{name}"))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "DELETE {name}");
    }
    // 65 chars — too long.
    let long = "a".repeat(65);
    let (status, _) = send(&router, get(&format!("/api/data/{long}"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn missing_doc_404() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    let (status, body) = send(&router, get("/api/data/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));
    assert!(body["error"].as_str().unwrap().contains("nope"));
}

#[tokio::test]
async fn put_get_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    let doc = json!({"title": "hello", "items": [1, 2, 3], "nested": {"a": null}});
    let (status, body) = send(&router, json_req("PUT", "/api/data/notes", &doc)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok": true}));

    let (status, body) = send(&router, get("/api/data/notes")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["data"], doc);

    // Replace.
    let doc2 = json!(["now", "an", "array"]);
    let (status, _) = send(&router, json_req("PUT", "/api/data/notes", &doc2)).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = send(&router, get("/api/data/notes")).await;
    assert_eq!(body["data"], doc2);

    // No stray tmp file after the atomic write.
    assert!(dir.path().join("notes.json").exists());
    assert!(!dir.path().join("notes.json.tmp").exists());
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

#[tokio::test]
async fn delete_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());
    let (status, _) = send(&router, json_req("PUT", "/api/data/tmp", &json!(1))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(&router, delete("/api/data/tmp")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok": true}));
    // Deleting again (missing) still succeeds.
    let (status, body) = send(&router, delete("/api/data/tmp")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"ok": true}));
    let (status, _) = send(&router, get("/api/data/tmp")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let router = app(dir.path());

    // Empty store (dir may not even exist yet) lists [].
    let (status, body) = send(&router, get("/api/data")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!([]));

    send(&router, json_req("PUT", "/api/data/beta", &json!({"b": 2}))).await;
    send(
        &router,
        json_req("PUT", "/api/data/alpha", &json!({"a": 1})),
    )
    .await;

    let (status, body) = send(&router, get("/api/data")).await;
    assert_eq!(status, StatusCode::OK);
    let docs = body["data"].as_array().unwrap();
    assert_eq!(docs.len(), 2);
    // Sorted by name.
    assert_eq!(docs[0]["name"], json!("alpha"));
    assert_eq!(docs[1]["name"], json!("beta"));
    for doc in docs {
        assert!(doc["bytes"].as_u64().unwrap() > 0);
        // modified = unix seconds as float, recent.
        let modified = doc["modified"].as_f64().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        assert!(modified > now - 60.0 && modified <= now + 1.0);
    }
}
