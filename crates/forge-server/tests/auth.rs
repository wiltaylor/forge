mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use forge_server::auth::{encode_token, jwt::unix_now};
use forge_server::{AuthConfig, Claims, ForgeApp};
use serde_json::json;

const SECRET: &str = "0123456789abcdef0123456789abcdef"; // 32 chars

fn auth_config() -> AuthConfig {
    AuthConfig::new(SECRET)
        .user("admin", "hunter2")
        .user_with_roles("ops", "opspass", vec!["ops".into(), "admin".into()])
}

fn app_with_auth() -> axum::Router {
    ForgeApp::new("auth-test")
        .auth(auth_config())
        .with_docstore(tempfile::tempdir().unwrap().keep())
        .router()
}

async fn login(router: &axum::Router, user: &str, pass: &str) -> (StatusCode, serde_json::Value) {
    send(
        router,
        json_req(
            "POST",
            "/api/auth/login",
            &json!({"username": user, "password": pass}),
        ),
    )
    .await
}

#[tokio::test]
async fn login_ok() {
    let router = app_with_auth();
    let (status, body) = login(&router, "admin", "hunter2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], json!(true));
    let data = &body["data"];
    assert!(!data["token"].as_str().unwrap().is_empty());
    assert_eq!(data["user"]["name"], json!("admin"));
    assert_eq!(data["user"]["roles"], json!([]));
    let expires_at = data["expires_at"].as_i64().unwrap();
    let expected = unix_now() + 86_400;
    assert!((expires_at - expected).abs() < 10, "expires_at ~ now+ttl");
}

#[tokio::test]
async fn login_roles_carried() {
    let router = app_with_auth();
    let (status, body) = login(&router, "ops", "opspass").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["user"]["roles"], json!(["ops", "admin"]));
    let token = body["data"]["token"].as_str().unwrap();
    let (status, body) = send(&router, get_bearer("/api/auth/me", token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["sub"], json!("ops"));
    assert_eq!(body["data"]["roles"], json!(["ops", "admin"]));
    assert_eq!(body["data"]["iss"], json!("forge"));
}

#[tokio::test]
async fn login_bad_credentials_401() {
    let router = app_with_auth();
    let (status, body) = login(&router, "admin", "wrong").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], json!(false));
    assert!(body["error"].is_string());

    let (status, _) = login(&router, "ghost", "hunter2").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_requires_token_when_auth_enabled() {
    let router = app_with_auth();
    let (status, body) = send(&router, get("/api/auth/me")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], json!(false));

    // Protected data routes too.
    let (status, _) = send(&router, get("/api/data")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_token_401() {
    let router = app_with_auth();
    let now = unix_now();
    let claims = Claims {
        sub: "admin".into(),
        roles: vec![],
        iat: now - 7200,
        exp: now - 3600,
        iss: None,
    };
    let token = encode_token(&claims, SECRET).unwrap();
    let (status, body) = send(&router, get_bearer("/api/auth/me", &token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], json!(false));
}

#[tokio::test]
async fn wrong_secret_401() {
    let router = app_with_auth();
    let claims = Claims::new("admin", vec![], 3600, None);
    let token = encode_token(&claims, "another-secret-another-secret-32").unwrap();
    let (status, _) = send(&router, get_bearer("/api/auth/me", &token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn query_param_token_accepted() {
    let router = app_with_auth();
    let (_, body) = login(&router, "admin", "hunter2").await;
    let token = body["data"]["token"].as_str().unwrap();
    let (status, body) = send(&router, get(&format!("/api/auth/me?token={token}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["sub"], json!("admin"));
}

#[tokio::test]
async fn argon2_hashed_user_verifies() {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"s3cret", &salt)
        .unwrap()
        .to_string();
    let cfg = AuthConfig::new(SECRET).user("h", hash);
    let router = ForgeApp::new("t").auth(cfg).router();
    let (status, _) = login(&router, "h", "s3cret").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = login(&router, "h", "nope").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn issuer_validated_only_when_configured() {
    let cfg = AuthConfig::new(SECRET).issuer("my-issuer").user("a", "b");
    let router = ForgeApp::new("t").auth(cfg).router();
    // Token without the configured issuer is rejected.
    let claims = Claims::new("a", vec![], 3600, Some("other".into()));
    let token = encode_token(&claims, SECRET).unwrap();
    let (status, _) = send(&router, get_bearer("/api/auth/me", &token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // Token minted by login carries and passes the issuer.
    let (_, body) = login(&router, "a", "b").await;
    let token = body["data"]["token"].as_str().unwrap();
    let (status, body) = send(&router, get_bearer("/api/auth/me", token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["iss"], json!("my-issuer"));
}

#[tokio::test]
async fn auth_disabled_everything_open_and_anonymous() {
    let dir = tempfile::tempdir().unwrap();
    let router = ForgeApp::new("open")
        .with_docstore(dir.path())
        .action("echo", |payload, _ctx| async move { Ok(payload) })
        .router();

    // Anonymous identity on /api/auth/me.
    let (status, body) = send(&router, get("/api/auth/me")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["sub"], json!("anonymous"));
    assert_eq!(body["data"]["roles"], json!([]));

    // Data and actions open without a token.
    let (status, _) = send(&router, get("/api/data")).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        &router,
        json_req("POST", "/api/actions/echo", &json!({"hi": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!({"hi": 1}));

    // Login is 404 when auth is disabled.
    let (status, body) = login(&router, "admin", "hunter2").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], json!(false));

    // Health reports auth_enabled=false.
    let (status, body) = send(&router, get("/api/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["auth_enabled"], json!(false));
}

#[tokio::test]
async fn short_secret_is_config_error() {
    let result = ForgeApp::new("t")
        .auth(AuthConfig::new("too-short"))
        .try_router();
    assert!(result.is_err());
}

#[tokio::test]
async fn health_reports_auth_enabled() {
    let router = app_with_auth();
    let (status, body) = send(&router, get("/api/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["auth_enabled"], json!(true));
    assert_eq!(body["data"]["app"], json!("auth-test"));
}

#[tokio::test]
async fn custom_route_can_extract_claims() {
    use axum::routing::get as axum_get;
    use forge_server::RequireClaims;
    let router = ForgeApp::new("t")
        .auth(auth_config())
        .route(
            "/api/whoami",
            axum_get(
                |RequireClaims(claims): RequireClaims| async move { forge_server::ok(claims.sub) },
            ),
        )
        .router();
    // No token → the RequireClaims extractor rejects with a 401 envelope.
    let (status, body) = send(&router, get("/api/whoami")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["ok"], json!(false));
    // With a token it resolves.
    let (_, body) = send(
        &router,
        json_req(
            "POST",
            "/api/auth/login",
            &json!({"username": "admin", "password": "hunter2"}),
        ),
    )
    .await;
    let token = body["data"]["token"].as_str().unwrap();
    let (status, body) = send(&router, get_bearer("/api/whoami", token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], json!("admin"));
}

#[tokio::test]
async fn header_wins_over_query_param() {
    let router = app_with_auth();
    let (_, body) = login(&router, "admin", "hunter2").await;
    let good = body["data"]["token"].as_str().unwrap();
    // Bad header + good query token → 401 (header wins).
    let req = Request::builder()
        .uri(format!("/api/auth/me?token={good}"))
        .header("authorization", "Bearer garbage")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&router, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
