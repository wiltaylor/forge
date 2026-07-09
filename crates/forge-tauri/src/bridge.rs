//! Pure request router: the frozen v1 contract semantics
//! (docs/api-contract.md) over any carrier. forge-server routes the same
//! paths with axum; `handle` mirrors it 1:1 so the parity suite's
//! expectations hold over IPC. Auth-disabled mode only: `me` answers with
//! anonymous claims and `login` is the contract-specified 404.

use serde::Serialize;
use serde_json::Value;

use forge_core::{
    err_value, health_payload, ok_empty_value, ok_value, unknown_action_error, ActionCtx, Claims,
    ForgeError,
};

use crate::state::ForgeState;

/// Transport-shaped response: the HTTP-equivalent status plus the Forge
/// envelope, exactly what the fetch-based client unwraps.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeResponse {
    pub status: u16,
    pub body: Value,
}

fn ok(data: impl Serialize) -> ForgeResponse {
    ForgeResponse {
        status: 200,
        body: ok_value(data),
    }
}

fn ok_empty() -> ForgeResponse {
    ForgeResponse {
        status: 200,
        body: ok_empty_value(),
    }
}

fn err(status: u16, message: impl Into<String>) -> ForgeResponse {
    ForgeResponse {
        status,
        body: err_value(message),
    }
}

fn err_forge(e: ForgeError) -> ForgeResponse {
    err(e.status(), e.to_string())
}

/// The router-fallback 404, same message shape as forge-server's frontend
/// fallback for `/api/*` misses.
fn not_found(path: &str) -> ForgeResponse {
    err(404, format!("not found: {path}"))
}

/// Route one request. `body` is the already-parsed JSON body (IPC carries
/// values, not byte streams, so the HTTP layer's "body is not valid JSON"
/// rejections cannot arise here).
pub async fn handle(
    state: &ForgeState,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> ForgeResponse {
    let method = method.to_ascii_uppercase();
    let Some(segments) = split_path(path) else {
        return not_found(path);
    };
    let parts: Vec<&str> = segments.iter().map(String::as_str).collect();

    match (method.as_str(), parts.as_slice()) {
        ("GET", ["api", "health"]) => ok(health_payload(
            &state.app,
            state.start.elapsed().as_secs_f64(),
            env!("CARGO_PKG_VERSION"),
            false,
            &state.action_names(),
        )),
        ("GET", ["api", "auth", "me"]) => ok(Claims::anonymous()),
        // Contract: 404 when auth is disabled — and over pure IPC it always is.
        ("POST", ["api", "auth", "login"]) => err(404, "auth is disabled"),
        ("GET", ["api", "data"]) => match &state.docstore {
            Some(store) => match store.list().await {
                Ok(docs) => ok(docs),
                Err(e) => err_forge(e),
            },
            None => not_found(path),
        },
        ("GET", ["api", "data", name]) => match &state.docstore {
            Some(store) => match store.get(name).await {
                Ok(doc) => ok(doc),
                Err(e) => err_forge(e),
            },
            None => not_found(path),
        },
        ("PUT", ["api", "data", name]) => match &state.docstore {
            // HTTP parity: an empty body stores JSON null.
            Some(store) => match store.put(name, &body.unwrap_or(Value::Null)).await {
                Ok(()) => ok_empty(),
                Err(e) => err_forge(e),
            },
            None => not_found(path),
        },
        ("DELETE", ["api", "data", name]) => match &state.docstore {
            Some(store) => match store.delete(name).await {
                Ok(()) => ok_empty(),
                Err(e) => err_forge(e),
            },
            None => not_found(path),
        },
        ("POST", ["api", "actions", name]) => run_action(state, name, body).await,
        _ => not_found(path),
    }
}

async fn run_action(state: &ForgeState, name: &str, body: Option<Value>) -> ForgeResponse {
    let Some(action) = state.actions.get(name) else {
        return err_forge(unknown_action_error(name, &state.action_names()));
    };
    // HTTP parity: an empty body dispatches an empty object.
    let payload = body.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let ctx = ActionCtx {
        claims: Claims::anonymous(),
        events: state.events.clone(),
    };
    match action(payload, ctx).await {
        Ok(data) => ok(data),
        Err(e) => err_forge(e),
    }
}

/// Split into percent-decoded segments (axum decodes path params; mirror it).
/// `None` on paths that cannot match any route (no leading `/`, bad escapes).
fn split_path(path: &str) -> Option<Vec<String>> {
    path.strip_prefix('/')?
        .split('/')
        .map(percent_decode)
        .collect()
}

/// Minimal percent-decoder — IPC paths carry no query strings.
fn percent_decode(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes.get(i + 1..i + 3)?;
            let hi = (hex[0] as char).to_digit(16)?;
            let lo = (hex[1] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

// Bridge parity tests: the transport-applicable subset of
// examples/parity/test_contract.py, plus IPC-specific routing edges.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use forge_core::{box_action, BoxedAction, DocStore, EventBus};
    use serde_json::json;

    use super::*;

    fn actions() -> BTreeMap<String, BoxedAction> {
        let mut map = BTreeMap::new();
        map.insert(
            "echo".to_string(),
            box_action(|payload, _ctx| async move { Ok(payload) }),
        );
        map.insert(
            "publish".to_string(),
            box_action(|payload: Value, ctx: ActionCtx| async move {
                let topic = payload["topic"].as_str().unwrap_or("events").to_string();
                ctx.events.publish(&topic, payload["data"].clone());
                Ok(json!({ "published": true, "topic": topic }))
            }),
        );
        map
    }

    fn state_with_store(dir: &std::path::Path) -> ForgeState {
        ForgeState::for_tests(Some(DocStore::new(dir)), actions())
    }

    #[tokio::test]
    async fn health_reports_capabilities() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_store(dir.path());
        let r = handle(&state, "GET", "/api/health", None).await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body["ok"], true);
        let data = &r.body["data"];
        assert_eq!(data["app"], "forge-tauri-test");
        assert_eq!(data["auth_enabled"], false);
        assert!(data["uptime_s"].is_number());
        assert_eq!(data["version"], env!("CARGO_PKG_VERSION"));
        let actions: Vec<_> = data["actions"].as_array().unwrap().to_vec();
        assert!(actions.contains(&json!("echo")));
    }

    #[tokio::test]
    async fn me_is_anonymous() {
        let state = ForgeState::for_tests(None, actions());
        let r = handle(&state, "GET", "/api/auth/me", None).await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body["data"]["sub"], "anonymous");
        assert_eq!(r.body["data"]["roles"], json!([]));
    }

    #[tokio::test]
    async fn login_is_contract_404() {
        let state = ForgeState::for_tests(None, actions());
        let r = handle(
            &state,
            "POST",
            "/api/auth/login",
            Some(json!({"username": "admin", "password": "admin"})),
        )
        .await;
        assert_eq!(r.status, 404);
        assert_eq!(r.body["ok"], false);
        assert_eq!(r.body["error"], "auth is disabled");
    }

    #[tokio::test]
    async fn doc_roundtrip_and_idempotent_delete() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_store(dir.path());
        let doc = json!({"n": 1, "nested": {"ok": true}, "s": "x"});

        let r = handle(&state, "PUT", "/api/data/paritytest-doc", Some(doc.clone())).await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body, json!({"ok": true}));

        let r = handle(&state, "GET", "/api/data/paritytest-doc", None).await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body["data"], doc);

        let r = handle(&state, "GET", "/api/data", None).await;
        let docs = r.body["data"].as_array().unwrap();
        let meta = docs
            .iter()
            .find(|d| d["name"] == "paritytest-doc")
            .expect("listed");
        assert!(meta["bytes"].as_u64().unwrap() > 0);
        assert!(meta["modified"].is_number());

        assert_eq!(
            handle(&state, "DELETE", "/api/data/paritytest-doc", None)
                .await
                .status,
            200
        );
        // Idempotent.
        assert_eq!(
            handle(&state, "DELETE", "/api/data/paritytest-doc", None)
                .await
                .status,
            200
        );
        let r = handle(&state, "GET", "/api/data/paritytest-doc", None).await;
        assert_eq!(r.status, 404);
        assert_eq!(r.body["ok"], false);
    }

    #[tokio::test]
    async fn empty_put_body_stores_null() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_store(dir.path());
        assert_eq!(
            handle(&state, "PUT", "/api/data/nul", None).await.status,
            200
        );
        let r = handle(&state, "GET", "/api/data/nul", None).await;
        assert_eq!(r.body["data"], Value::Null);
    }

    #[tokio::test]
    async fn bad_doc_names_reject() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_store(dir.path());
        for bad in ["UPPER", "-lead", ".dot", &"a".repeat(70), "a b"] {
            let r = handle(&state, "PUT", &format!("/api/data/{bad}"), Some(json!({}))).await;
            assert_eq!(r.status, 400, "{bad:?}");
            assert_eq!(r.body["ok"], false);
        }
        // Path separators never reach the name (route shape mismatch), and
        // percent-encoded traversal decodes into an invalid name.
        let r = handle(&state, "PUT", "/api/data/sl/ash", Some(json!({}))).await;
        assert_eq!(r.status, 404);
        let r = handle(&state, "PUT", "/api/data/%2e%2e", Some(json!({}))).await;
        assert_eq!(r.status, 400);
    }

    #[tokio::test]
    async fn docstore_disabled_is_404() {
        let state = ForgeState::for_tests(None, actions());
        assert_eq!(handle(&state, "GET", "/api/data", None).await.status, 404);
        assert_eq!(handle(&state, "GET", "/api/data/x", None).await.status, 404);
    }

    #[tokio::test]
    async fn action_echo_roundtrip() {
        let state = ForgeState::for_tests(None, actions());
        let payload = json!({"ping": "pong", "n": 3});
        let r = handle(&state, "POST", "/api/actions/echo", Some(payload.clone())).await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body["data"], payload);

        // Empty body dispatches an empty object (HTTP parity).
        let r = handle(&state, "POST", "/api/actions/echo", None).await;
        assert_eq!(r.body["data"], json!({}));
    }

    #[tokio::test]
    async fn unknown_action_names_the_registry() {
        let state = ForgeState::for_tests(None, actions());
        let r = handle(
            &state,
            "POST",
            "/api/actions/definitely-not-registered",
            Some(json!({})),
        )
        .await;
        assert_eq!(r.status, 404);
        assert_eq!(r.body["ok"], false);
        let msg = r.body["error"].as_str().unwrap();
        assert!(msg.contains("echo"), "{msg}");
        assert!(msg.contains("definitely-not-registered"), "{msg}");
    }

    #[tokio::test]
    async fn action_publishes_to_event_bus() {
        let state = ForgeState::for_tests(None, actions());
        let mut rx = state.events.subscribe();
        let r = handle(
            &state,
            "POST",
            "/api/actions/publish",
            Some(json!({"topic": "paritytest", "data": {"hello": "ipc"}})),
        )
        .await;
        assert_eq!(r.status, 200);
        let event = rx.try_recv().expect("event published");
        assert_eq!(event.topic, "paritytest");
        assert_eq!(
            serde_json::from_str::<Value>(&event.json).unwrap(),
            json!({"hello": "ipc"})
        );
    }

    #[tokio::test]
    async fn api_miss_is_json_404() {
        let state = ForgeState::for_tests(None, actions());
        let r = handle(&state, "GET", "/api/definitely-not-a-route", None).await;
        assert_eq!(r.status, 404);
        assert_eq!(r.body["ok"], false);
        assert_eq!(r.body["error"], "not found: /api/definitely-not-a-route");
        // Wrong method on a real path is a miss too.
        let r = handle(&state, "DELETE", "/api/health", None).await;
        assert_eq!(r.status, 404);
    }
}
