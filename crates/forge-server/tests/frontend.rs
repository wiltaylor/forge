mod common;

use axum::http::StatusCode;
use common::*;
use forge_server::ForgeApp;
use serde_json::json;

fn write_dist(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><html><body>forge-index</body></html>",
    )
    .unwrap();
    std::fs::write(dir.join("assets/app-abc123.js"), "console.log('app');").unwrap();
    std::fs::write(dir.join("favicon.svg"), "<svg></svg>").unwrap();
}

#[tokio::test]
async fn dir_mode_serves_files_and_spa_fallback() {
    let dir = tempfile::tempdir().unwrap();
    write_dist(dir.path());
    let router = ForgeApp::new("fe-test").frontend_dir(dir.path()).router();

    // Root serves index.html.
    let (status, headers, body) = send_raw(&router, get("/")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("forge-index"));
    assert!(headers
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/html"));

    // Exact asset match with mime type + immutable caching.
    let (status, headers, body) = send_raw(&router, get("/assets/app-abc123.js")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("console.log"));
    let ct = headers.get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("javascript"), "content-type was {ct}");
    let cc = headers.get("cache-control").unwrap().to_str().unwrap();
    assert!(cc.contains("immutable"), "cache-control was {cc}");

    // Non-asset files are revalidated.
    let (status, headers, _) = send_raw(&router, get("/favicon.svg")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("cache-control").unwrap(), "no-cache");

    // Unknown non-/api path falls back to index.html (SPA).
    let (status, _, body) = send_raw(&router, get("/settings/profile")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("forge-index"));
}

#[tokio::test]
async fn api_misses_stay_json_404() {
    let dir = tempfile::tempdir().unwrap();
    write_dist(dir.path());
    let router = ForgeApp::new("fe-test").frontend_dir(dir.path()).router();

    let (status, body) = send(&router, get("/api/unknown")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));
    assert!(body["error"].as_str().unwrap().contains("/api/unknown"));

    // Deeper misses too.
    let (status, body) = send(&router, get("/api/data/x/y/z")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));
}

#[tokio::test]
async fn no_frontend_configured_404s() {
    let router = ForgeApp::new("bare").router();
    let (status, body) = send(&router, get("/anything")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));
}

#[cfg(feature = "embed")]
mod embedded {
    use super::*;

    #[derive(rust_embed::RustEmbed)]
    #[folder = "tests/fixtures/dist"]
    struct Assets;

    fn app() -> axum::Router {
        ForgeApp::new("embed-test")
            .frontend_embedded::<Assets>()
            .router()
    }

    #[tokio::test]
    async fn serves_embedded_assets() {
        let router = app();
        let (status, headers, body) = send_raw(&router, get("/")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("embedded-index"));
        assert!(headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));

        let (status, headers, body) = send_raw(&router, get("/assets/app-e5f6.js")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("embedded"));
        assert!(headers
            .get("cache-control")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("immutable"));
    }

    #[tokio::test]
    async fn spa_fallback_and_api_404() {
        let router = app();
        let (status, _, body) = send_raw(&router, get("/deep/client/route")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("embedded-index"));

        let (status, body) = send(&router, get("/api/none")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["ok"], json!(false));
    }
}
