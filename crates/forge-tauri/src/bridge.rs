//! Pure request router: the frozen v1 contract semantics
//! (docs/api-contract.md) over any carrier. forge-server routes the same
//! paths with axum; this module mirrors it 1:1, so the contract corpus
//! (`contract/corpus.json`) runs over IPC unchanged.
//!
//! Auth works the way it does over HTTP: with an [`Auth`] configured the
//! protected routes need a valid token, and login mints one. With no `Auth`
//! at all every route is open and handlers see [`Claims::anonymous`].
//! The token arrives as an argument rather than a header — that is the only
//! difference the carrier makes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use forge_core::{
    err_value, health_payload, ok_empty_value, ok_value, unknown_action_error, ActionCtx, Auth,
    Claims, Components, DocStore, ForgeError, MeResponse,
};

use crate::state::ForgeState;

/// Transport-shaped response: the HTTP-equivalent status plus the Forge
/// envelope, exactly what the fetch-based client unwraps.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeResponse {
    pub status: u16,
    pub body: Value,
}

impl ForgeState {
    /// Route one contract request and answer it.
    ///
    /// `body` is the already-parsed JSON body (IPC carries values, not byte
    /// streams, so the HTTP layer's "body is not valid JSON" rejections
    /// cannot arise here). `token` is the bearer token, if the caller has
    /// one — the IPC equivalent of `Authorization: Bearer`.
    ///
    /// Public so a host without a Tauri runtime can drive the same contract;
    /// the plugin's `request` command is one caller and the contract-corpus
    /// driver is another.
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> ForgeResponse {
        let method = method.to_ascii_uppercase();
        let Some(segments) = split_path(path) else {
            return not_found(path);
        };
        let parts: Vec<&str> = segments.iter().map(String::as_str).collect();
        let Some(route) = resolve(self, &method, &parts) else {
            return not_found(path);
        };

        let claims = if route.is_open() {
            Claims::anonymous()
        } else {
            match authenticate(self.auth.as_ref(), token) {
                Ok(claims) => claims,
                Err(e) => return err_forge(e),
            }
        };
        dispatch(self, route, claims, body).await
    }
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

/// One resolved route. Resolving before authenticating is what keeps a miss a
/// 404 rather than a 401: forge-server's fallback sits outside the auth
/// middleware, so an unknown path never asks for a token.
///
/// A route that needs a configured feature carries it, so a route that
/// resolved without one cannot be built.
enum Route<'a> {
    Health,
    Login,
    Me,
    DataList(&'a DocStore),
    DataGet(&'a DocStore, &'a str),
    DataPut(&'a DocStore, &'a str),
    DataDelete(&'a DocStore, &'a str),
    Action(&'a str),
    Components(&'a Components),
}

impl Route<'_> {
    /// Whether the route answers without a token. forge-server mounts these
    /// two outside the auth middleware; everything else is behind it.
    fn is_open(&self) -> bool {
        matches!(self, Route::Health | Route::Login)
    }
}

/// Match a request onto a route. A route whose feature is not configured does
/// not exist, mirroring forge-server mounting it only when it is.
fn resolve<'a>(state: &'a ForgeState, method: &str, parts: &[&'a str]) -> Option<Route<'a>> {
    let docs = state.docstore.as_ref();
    Some(match (method, parts) {
        ("GET", ["api", "health"]) => Route::Health,
        ("POST", ["api", "auth", "login"]) => Route::Login,
        ("GET", ["api", "auth", "me"]) => Route::Me,
        ("GET", ["api", "data"]) => Route::DataList(docs?),
        ("GET", ["api", "data", name]) => Route::DataGet(docs?, name),
        ("PUT", ["api", "data", name]) => Route::DataPut(docs?, name),
        ("DELETE", ["api", "data", name]) => Route::DataDelete(docs?, name),
        ("POST", ["api", "actions", name]) => Route::Action(name),
        ("GET", ["api", "components"]) => Route::Components(state.components.as_ref()?),
        _ => return None,
    })
}

/// The identity behind a request. With no [`Auth`] configured every request is
/// anonymous — auth-disabled mode is first-class in the contract.
fn authenticate(auth: Option<&Auth>, token: Option<&str>) -> Result<Claims, ForgeError> {
    let Some(auth) = auth else {
        return Ok(Claims::anonymous());
    };
    let Some(token) = token.filter(|t| !t.is_empty()) else {
        return Err(ForgeError::Unauthorized(
            "missing token (pass it as the request command's `token` argument)".into(),
        ));
    };
    auth.validate(token)
}

async fn dispatch(
    state: &ForgeState,
    route: Route<'_>,
    claims: Claims,
    body: Option<Value>,
) -> ForgeResponse {
    match route {
        Route::Health => ok(health_payload(
            &state.app,
            state.start.elapsed().as_secs_f64(),
            env!("CARGO_PKG_VERSION"),
            state.auth.is_some(),
            &state.action_names(),
        )),
        Route::Login => login(state, body),
        // Contract: the decoded claims, which is `{sub, roles, iss, exp}` —
        // the same payload forge-server answers with.
        Route::Me => ok(MeResponse::from(&claims)),
        Route::DataList(store) => match store.list().await {
            Ok(docs) => ok(docs),
            Err(e) => err_forge(e),
        },
        Route::DataGet(store, name) => match store.get(name).await {
            Ok(doc) => ok(doc),
            Err(e) => err_forge(e),
        },
        // HTTP parity: an empty body stores JSON null.
        Route::DataPut(store, name) => match store.put(name, &body.unwrap_or(Value::Null)).await {
            Ok(()) => ok_empty(),
            Err(e) => err_forge(e),
        },
        Route::DataDelete(store, name) => match store.delete(name).await {
            Ok(()) => ok_empty(),
            Err(e) => err_forge(e),
        },
        Route::Action(name) => run_action(state, name, claims, body).await,
        Route::Components(components) => match components.manifest(&state.app).await {
            Ok(manifest) => ok(manifest),
            Err(e) => err_forge(e),
        },
    }
}

#[derive(Debug, Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

/// Contract: 404 when auth is disabled, and in external-issuer mode (a
/// validator without a login config) there is no login endpoint either. Both
/// are decided before the body is read — with no endpoint here, the body is
/// nobody's business.
fn login(state: &ForgeState, body: Option<Value>) -> ForgeResponse {
    let Some(auth) = state.auth.as_ref().filter(|a| a.can_login()) else {
        return err(404, forge_core::auth::AUTH_DISABLED);
    };
    let body = match serde_json::from_value::<LoginBody>(body.unwrap_or(Value::Null)) {
        Ok(body) => body,
        Err(e) => {
            return err(
                400,
                format!("body must be JSON {{username, password}}: {e}"),
            )
        }
    };
    match auth.login(&body.username, &body.password) {
        Ok(response) => ok(response),
        Err(e) => err_forge(e),
    }
}

async fn run_action(
    state: &ForgeState,
    name: &str,
    claims: Claims,
    body: Option<Value>,
) -> ForgeResponse {
    let Some(action) = state.actions.get(name) else {
        return err_forge(unknown_action_error(name, &state.action_names()));
    };
    // HTTP parity: an empty body dispatches an empty object.
    let payload = body.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let ctx = ActionCtx {
        claims,
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

/// What the contract corpus cannot state for this transport. Everything the
/// corpus does cover runs in `tests/corpus.rs` — nothing here restates it.
///
/// - The routing miss. The corpus case asserts a `content-type` header, which
///   an IPC response has no room for, so it is declared inapplicable.
/// - What a rejected doc name leaves on disk. The corpus reads envelopes; that
///   nothing was written is a fact about the directory behind them.
/// - The identity an action is handed. The corpus fixture's actions do not
///   look at their caller.
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Builder;

    fn state() -> ForgeState {
        Builder::new("forge-tauri-test")
            .action("echo", |payload, _ctx| async move { Ok(payload) })
            .try_state()
            .expect("state")
    }

    /// A miss is a 404 envelope, whether the path is unknown, the method is
    /// wrong, or the feature behind the path was never configured.
    #[tokio::test]
    async fn a_miss_is_a_json_404() {
        let state = state();
        for (method, path) in [
            ("GET", "/api/definitely-not-a-route"),
            ("DELETE", "/api/health"),
            // No doc store and no components dir on this builder.
            ("GET", "/api/data"),
            ("GET", "/api/data/x"),
            ("GET", "/api/components"),
            // Not a path at all.
            ("GET", "no-leading-slash"),
        ] {
            let r = state.request(method, path, None, None).await;
            assert_eq!(r.status, 404, "{method} {path}");
            assert_eq!(r.body["ok"], false, "{method} {path}");
        }
    }

    /// Percent-decoding happens before the doc-name rule sees the name, the
    /// way axum decodes a path parameter — otherwise `%2E%2E` would be stored
    /// verbatim as a name the rule never inspected.
    #[tokio::test]
    async fn a_doc_name_is_percent_decoded_before_it_is_validated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = Builder::new("forge-tauri-test")
            .with_docstore(dir.path())
            .try_state()
            .expect("state");

        let r = state
            .request("PUT", "/api/data/%2E%2E", Some(json!({})), None)
            .await;
        assert_eq!(r.status, 400);
        assert_eq!(r.body["ok"], false);
        assert!(!dir.path().join("%2E%2E.json").exists());
    }

    /// An action sees the identity that made the request, not a hardcoded one.
    #[tokio::test]
    async fn an_action_receives_the_callers_claims() {
        let state = Builder::new("forge-tauri-test")
            .auth(forge_core::AuthConfig::new("0123456789abcdef0123456789abcdef").user("ann", "pw"))
            .action("whoami", |_payload, ctx: ActionCtx| async move {
                Ok(json!({ "sub": ctx.claims.sub }))
            })
            .try_state()
            .expect("state");

        let login = state
            .request(
                "POST",
                "/api/auth/login",
                Some(json!({"username": "ann", "password": "pw"})),
                None,
            )
            .await;
        assert_eq!(login.status, 200);
        let token = login.body["data"]["token"].as_str().expect("token");

        let r = state
            .request("POST", "/api/actions/whoami", None, Some(token))
            .await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body["data"]["sub"], "ann");
    }
}
