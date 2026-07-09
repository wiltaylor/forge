//! Minimal relying party for manually testing forge-auth SSO.
//!
//! Register a client in forge-auth (redirect URI http://127.0.0.1:9000/cb),
//! then:
//!   DEMO_CLIENT_ID=demo DEMO_CLIENT_SECRET=... FORGE_AUTH_URL=http://127.0.0.1:8770 \
//!     cargo run -p auth-demo
//! and open http://127.0.0.1:9000 — "Sign in" bounces through forge-auth and
//! the callback page dumps the verified-token claims.

use std::collections::HashMap;
use std::sync::Mutex;

use axum::extract::Query;
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::get;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use forge_server::ForgeApp;
use rand::RngCore;
use sha2::{Digest, Sha256};

static PKCE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn cfg(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn random() -> String {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

async fn index() -> Html<String> {
    Html("<h1>auth-demo</h1><p><a href=\"/login\">Sign in with forge-auth</a></p>".into())
}

async fn login() -> Redirect {
    let auth = cfg("FORGE_AUTH_URL", "http://127.0.0.1:8770");
    let client_id = cfg("DEMO_CLIENT_ID", "demo");
    let state = random();
    let verifier = random();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    PKCE.lock().unwrap().get_or_insert_with(HashMap::new).insert(state.clone(), verifier);
    Redirect::to(&format!(
        "{auth}/oauth2/authorize?response_type=code&client_id={client_id}\
         &redirect_uri=http%3A%2F%2F127.0.0.1%3A9000%2Fcb&scope=openid%20profile%20email%20roles\
         &state={state}&code_challenge={challenge}&code_challenge_method=S256"
    ))
}

async fn callback(Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    let auth = cfg("FORGE_AUTH_URL", "http://127.0.0.1:8770");
    let (Some(code), Some(state)) = (q.get("code"), q.get("state")) else {
        return Html(format!("<h1>Error</h1><pre>{q:?}</pre>"));
    };
    let Some(verifier) = PKCE.lock().unwrap().get_or_insert_with(HashMap::new).remove(state) else {
        return Html("<h1>Error</h1><p>unknown state</p>".into());
    };

    let tokens: serde_json::Value = reqwest::Client::new()
        .post(format!("{auth}/oauth2/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", "http://127.0.0.1:9000/cb"),
            ("code_verifier", &verifier),
            ("client_id", &cfg("DEMO_CLIENT_ID", "demo")),
            ("client_secret", &cfg("DEMO_CLIENT_SECRET", "")),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Ask the IdP who this is (server-side validation of the access token).
    let userinfo: serde_json::Value = match tokens["access_token"].as_str() {
        Some(at) => reqwest::Client::new()
            .get(format!("{auth}/oauth2/userinfo"))
            .bearer_auth(at)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap(),
        None => serde_json::json!({"error": "no access_token", "response": tokens}),
    };

    Html(format!(
        "<h1>Signed in via forge-auth</h1><h2>userinfo</h2><pre>{}</pre><h2>token response</h2><pre>{}</pre>",
        serde_json::to_string_pretty(&userinfo).unwrap(),
        serde_json::to_string_pretty(&tokens).unwrap(),
    ))
}

#[tokio::main]
async fn main() -> Result<(), forge_server::ForgeError> {
    std::env::set_var("FORGE_PORT", cfg("DEMO_PORT", "9000"));
    ForgeApp::new("auth-demo")
        .route("/", get(index))
        .route("/login", get(login))
        .route("/cb", get(callback))
        .serve()
        .await
}
