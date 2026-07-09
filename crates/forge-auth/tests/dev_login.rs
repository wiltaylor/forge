//! The dev-login surface must exist with the feature and be completely
//! absent without it. Run both ways:
//!   cargo test --test dev_login
//!   cargo test --test dev_login --features dev-login

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use forge_auth::config::Config;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn test_app() -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        issuer: "http://idp.test".into(),
        database_url: format!("sqlite://{}?mode=rwc", dir.path().join("t.db").display()),
        cookie_secure: false,
        access_ttl: 900,
        refresh_ttl: 3600,
        session_idle_ttl: 3600,
        session_absolute_ttl: 7200,
        admin_user: "admin".into(),
        admin_password: Some("admin-pw-123".into()),
        seed_file: None,
        host: "127.0.0.1".into(),
        port: 0,
    };
    let state = forge_auth::init_state(cfg).await.unwrap();
    let router = forge_auth::app::build_router(state, forge_server::ForgeApp::new("test")).unwrap();
    (router, dir)
}

async fn get_json(router: &Router, uri: &str) -> (StatusCode, Value) {
    let res = router
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[cfg(not(feature = "dev-login"))]
#[tokio::test]
async fn dev_login_absent_in_default_build() {
    let (router, _dir) = test_app().await;
    let (status, _) = get_json(&router, "/api/login/dev/users").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, session) = get_json(&router, "/api/session").await;
    assert_eq!(session["data"]["dev_login"], false);

    // No seeded dev users either.
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"username": "dev-admin", "password": ""}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "dev-login")]
#[tokio::test]
async fn dev_login_one_click_flow() {
    let (router, _dir) = test_app().await;

    let (status, users) = get_json(&router, "/api/login/dev/users").await;
    assert_eq!(status, StatusCode::OK);
    let list = users["data"]["users"].as_array().unwrap();
    assert_eq!(list.len(), 3);
    let dev_admin = list
        .iter()
        .find(|u| u["username"] == "dev-admin")
        .expect("dev-admin seeded");

    // One-click login → session cookie with the dev amr.
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login/dev")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"user_id": dev_admin["id"]}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let session: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(session["data"]["dev_login"], true);
    assert_eq!(session["data"]["user"]["username"], "dev-admin");
    assert_eq!(session["data"]["user"]["amr"], serde_json::json!(["dev"]));
    assert!(session["data"]["user"]["roles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("admin")));
}
