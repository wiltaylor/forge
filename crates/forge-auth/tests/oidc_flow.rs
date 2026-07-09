//! End-to-end OIDC golden path against the real router on temp SQLite:
//! login → authorize (PKCE) → token → JWKS-verified JWT → userinfo →
//! refresh rotation + reuse detection → RFC 8693 exchange → revocation.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use forge_auth::config::Config;
use forge_auth::util::{b64url, pkce_s256};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::Digest;
use tower::ServiceExt;

struct TestApp {
    router: Router,
    _dir: tempfile::TempDir,
}

async fn test_app() -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let cfg = Config {
        issuer: "http://idp.test".into(),
        database_url: format!("sqlite://{}?mode=rwc", db_path.display()),
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
    TestApp { router, _dir: dir }
}

async fn send(router: &Router, req: Request<Body>) -> (StatusCode, Value, Vec<String>) {
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let cookies: Vec<String> = res
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
        .collect();
    let location = res
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let mut body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if let Some(loc) = location {
        body["__location"] = json!(loc);
    }
    (status, body, cookies)
}

fn json_req(method: &str, uri: &str, cookie: Option<&str>, body: Value) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forge-auth", "1");
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

fn form_req(uri: &str, pairs: &[(&str, &str)]) -> Request<Body> {
    let body: String = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap()
}

async fn login_admin(router: &Router) -> String {
    let (status, body, cookies) = send(
        router,
        json_req(
            "POST",
            "/api/login",
            None,
            json!({"username": "admin", "password": "admin-pw-123"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login failed: {body}");
    cookies
        .into_iter()
        .find(|c| c.starts_with("forge_auth_session="))
        .expect("session cookie")
}

async fn create_client(router: &Router, cookie: &str, overrides: Value) -> (String, String) {
    let mut body = json!({
        "id": "test-app",
        "name": "Test App",
        "redirect_uris": ["http://rp.test/cb"],
        "trusted": true,
    });
    if let (Some(base), Some(extra)) = (body.as_object_mut(), overrides.as_object()) {
        for (k, v) in extra {
            base.insert(k.clone(), v.clone());
        }
    }
    let (status, res, _) = send(router, json_req("POST", "/api/admin/clients", Some(cookie), body)).await;
    assert_eq!(status, StatusCode::OK, "client create failed: {res}");
    let id = res["data"]["id"].as_str().unwrap().to_string();
    let secret = res["data"]["client_secret"].as_str().unwrap_or("").to_string();
    (id, secret)
}

/// Run /oauth2/authorize with a session until it redirects to the RP with a code.
async fn get_code(router: &Router, cookie: &str, client_id: &str, challenge: &str) -> String {
    let uri = format!(
        "/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri=http%3A%2F%2Frp.test%2Fcb&scope=openid%20profile%20email%20roles&state=xyz&nonce=n0nce&code_challenge={challenge}&code_challenge_method=S256"
    );
    let req = Request::builder()
        .uri(&uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    let (status, body, _) = send(router, req).await;
    assert_eq!(status, StatusCode::SEE_OTHER, "authorize did not redirect: {body}");
    let location = body["__location"].as_str().unwrap();
    assert!(location.starts_with("http://rp.test/cb"), "unexpected redirect {location}");
    let url = url::Url::parse(location).unwrap();
    let code = url.query_pairs().find(|(k, _)| k == "code").map(|(_, v)| v.into_owned());
    assert_eq!(
        url.query_pairs().find(|(k, _)| k == "state").map(|(_, v)| v.into_owned()),
        Some("xyz".to_string())
    );
    code.expect("code in redirect")
}

fn decode_jwt_no_verify(token: &str) -> Value {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    let payload = token.split('.').nth(1).unwrap();
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).unwrap()).unwrap()
}

#[tokio::test]
async fn full_oidc_flow() {
    let app = test_app().await;
    let router = &app.router;
    let cookie = login_admin(router).await;

    // Client that may also exchange tokens for the "other-app" audience.
    let (client_id, client_secret) =
        create_client(router, &cookie, json!({"exchange_audiences": ["other-app"]})).await;
    let (other_id, _) = create_client(
        router,
        &cookie,
        json!({"id": "other-app", "name": "Other App", "role_mappings": {"admin": "boss"}}),
    )
    .await;

    // --- authorization code + PKCE ---
    let verifier = "a".repeat(43);
    let challenge = pkce_s256(&verifier);
    let code = get_code(router, &cookie, &client_id, &challenge).await;

    // Code replay protection: exchange once, then the same code must fail.
    let (status, tokens, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", "http://rp.test/cb"),
                ("code_verifier", &verifier),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "token exchange failed: {tokens}");
    let access = tokens["access_token"].as_str().unwrap().to_string();
    let refresh = tokens["refresh_token"].as_str().unwrap().to_string();
    let id_token = tokens["id_token"].as_str().unwrap().to_string();

    let (status, err, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", "http://rp.test/cb"),
                ("code_verifier", &verifier),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "invalid_grant");

    // --- verify the JWT against the published JWKS ---
    let (_, jwks, _) = send(router, Request::builder().uri("/.well-known/jwks.json").body(Body::empty()).unwrap()).await;
    let jwk = &jwks["keys"][0];
    let key = jsonwebtoken::DecodingKey::from_rsa_components(
        jwk["n"].as_str().unwrap(),
        jwk["e"].as_str().unwrap(),
    )
    .unwrap();
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&["http://idp.test"]);
    validation.set_audience(&[&client_id]);
    let verified =
        jsonwebtoken::decode::<Value>(&access, &key, &validation).expect("JWKS-verified access token");
    assert_eq!(verified.claims["roles"], json!(["admin"]));
    assert_eq!(verified.claims["preferred_username"], "admin");

    let id_claims = decode_jwt_no_verify(&id_token);
    assert_eq!(id_claims["nonce"], "n0nce");
    assert_eq!(id_claims["aud"], client_id);

    // --- userinfo ---
    let req = Request::builder()
        .uri("/oauth2/userinfo")
        .header(header::AUTHORIZATION, format!("Bearer {access}"))
        .body(Body::empty())
        .unwrap();
    let (status, userinfo, _) = send(router, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(userinfo["preferred_username"], "admin");
    assert_eq!(userinfo["roles"], json!(["admin"]));

    // --- refresh rotation ---
    let (status, refreshed, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "refresh failed: {refreshed}");
    let refresh2 = refreshed["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(refresh, refresh2, "refresh token must rotate");

    // Reuse of the old token revokes the family: the NEW token dies too.
    let (status, err, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "invalid_grant");
    let (status, err, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh2),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "family revocation must kill rotated token: {err}");

    // --- RFC 8693 token exchange with role mapping for the target ---
    let (status, exchanged, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "urn:ietf:params:oauth:grant-type:token-exchange"),
                ("subject_token", &access),
                ("subject_token_type", "urn:ietf:params:oauth:token-type:access_token"),
                ("audience", &other_id),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "exchange failed: {exchanged}");
    let ex_claims = decode_jwt_no_verify(exchanged["access_token"].as_str().unwrap());
    assert_eq!(ex_claims["aud"], other_id);
    assert_eq!(ex_claims["azp"], client_id);
    assert_eq!(ex_claims["roles"], json!(["boss"]), "target role mapping applied");

    // Exchange for a non-allowed audience must fail.
    let (status, err, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "urn:ietf:params:oauth:grant-type:token-exchange"),
                ("subject_token", &access),
                ("audience", "unrelated-app"),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "invalid_target", "{err}");
}

#[tokio::test]
async fn pkce_and_client_auth_are_enforced() {
    let app = test_app().await;
    let router = &app.router;
    let cookie = login_admin(router).await;
    let (client_id, client_secret) = create_client(router, &cookie, json!({})).await;

    let verifier = "b".repeat(43);
    let code = get_code(router, &cookie, &client_id, &pkce_s256(&verifier)).await;

    // Wrong verifier fails.
    let (status, err, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", "http://rp.test/cb"),
                ("code_verifier", &"c".repeat(43)),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "invalid_grant");

    // Wrong client secret fails with invalid_client.
    let (status, err, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", "http://rp.test/cb"),
                ("code_verifier", &verifier),
                ("client_id", &client_id),
                ("client_secret", "wrong"),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(err["error"], "invalid_client");
}

#[tokio::test]
async fn authorize_requires_login_and_registered_redirect() {
    let app = test_app().await;
    let router = &app.router;
    let cookie = login_admin(router).await;
    let (client_id, _) = create_client(router, &cookie, json!({})).await;

    // No session → redirect to the hosted login page with a request id.
    let uri = format!(
        "/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri=http%3A%2F%2Frp.test%2Fcb&scope=openid&code_challenge={}&code_challenge_method=S256",
        pkce_s256(&"d".repeat(43))
    );
    let (status, body, _) = send(router, Request::builder().uri(&uri).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let location = body["__location"].as_str().unwrap();
    assert!(location.starts_with("/login?request="), "{location}");

    // The login request info endpoint knows the client.
    let request_id = location.split('=').nth(1).unwrap();
    let (status, info, _) = send(
        router,
        Request::builder()
            .uri(format!("/api/login/request/{request_id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["data"]["client_name"], "Test App");

    // Unregistered redirect_uri → hard 400, no redirect.
    let uri = format!(
        "/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri=http%3A%2F%2Fevil.test%2Fcb&scope=openid"
    );
    let req = Request::builder()
        .uri(&uri)
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn legacy_hs256_exchange_for_forge_apps() {
    let app = test_app().await;
    let router = &app.router;
    let cookie = login_admin(router).await;
    let legacy_secret = "a-shared-forge-secret-of-32-chars!!";
    let (client_id, client_secret) =
        create_client(router, &cookie, json!({"exchange_audiences": ["legacy-app"]})).await;
    create_client(
        router,
        &cookie,
        json!({"id": "legacy-app", "name": "Legacy", "legacy_hs256_secret": legacy_secret}),
    )
    .await;

    let verifier = "e".repeat(43);
    let code = get_code(router, &cookie, &client_id, &pkce_s256(&verifier)).await;
    let (_, tokens, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", "http://rp.test/cb"),
                ("code_verifier", &verifier),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    let access = tokens["access_token"].as_str().unwrap().to_string();

    let (status, exchanged, _) = send(
        router,
        form_req(
            "/oauth2/token",
            &[
                ("grant_type", "urn:ietf:params:oauth:grant-type:token-exchange"),
                ("subject_token", &access),
                ("audience", "legacy-app"),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{exchanged}");
    let legacy = exchanged["access_token"].as_str().unwrap();

    // Verify with the shared secret exactly like forge-server does (HS256,
    // sub = username).
    let key = jsonwebtoken::DecodingKey::from_secret(legacy_secret.as_bytes());
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_aud = false;
    let claims = jsonwebtoken::decode::<Value>(legacy, &key, &validation).unwrap().claims;
    assert_eq!(claims["sub"], "admin");
    assert_eq!(claims["roles"], json!(["admin"]));
}

#[tokio::test]
async fn consent_flow_for_untrusted_clients() {
    let app = test_app().await;
    let router = &app.router;
    let cookie = login_admin(router).await;
    let (client_id, _) = create_client(router, &cookie, json!({"trusted": false, "client_type": "public"})).await;

    let uri = format!(
        "/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri=http%3A%2F%2Frp.test%2Fcb&scope=openid&code_challenge={}&code_challenge_method=S256",
        pkce_s256(&"f".repeat(43))
    );
    let req = Request::builder().uri(&uri).header(header::COOKIE, &cookie).body(Body::empty()).unwrap();
    let (status, body, _) = send(router, req).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let location = body["__location"].as_str().unwrap().to_string();
    assert!(location.starts_with("/consent?request="), "{location}");
    let request_id = location.split('=').nth(1).unwrap().to_string();

    // Approve via the consent API, resume authorize, get the code.
    let (status, decided, _) = send(
        router,
        json_req(
            "POST",
            &format!("/api/consent/{request_id}"),
            Some(&cookie),
            json!({"approve": true}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    let resume = decided["data"]["redirect_to"].as_str().unwrap().to_string();
    let req = Request::builder().uri(&resume).header(header::COOKIE, &cookie).body(Body::empty()).unwrap();
    let (status, body, _) = send(router, req).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(body["__location"].as_str().unwrap().contains("code="), "{body}");
}

#[tokio::test]
async fn sha2_helper_matches_hex() {
    // Guard the token-hash format the DB relies on.
    let digest = hex::encode(sha2::Sha256::digest("abc"));
    assert_eq!(digest, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(forge_auth::util::sha256_hex("abc"), digest);
    let _ = b64url(b"x");
}
