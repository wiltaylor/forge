//! The [`ForgeApp`] builder — assembles state, routes, auth wiring, CORS and
//! the static frontend into an [`axum::Router`], and serves it.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use axum::http::{header, HeaderValue, Method};
use axum::routing::{get, post, MethodRouter};
use axum::Router;
use serde_json::Value;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::actions::{box_action, ActionCtx, BoxedAction};
use crate::auth::{AuthConfig, AuthState, Hs256Validator, TokenValidator};
use crate::docstore::DocStore;
use crate::error::ForgeError;
use crate::events::EventBus;
use crate::frontend::Frontend;
use crate::state::{ForgeState, StateInner};
use crate::{auth, components, docstore, events, frontend, health};

pub(crate) const DEFAULT_CORS_ORIGINS: &str = "http://localhost:5173,http://127.0.0.1:5173";

/// Builder for a Forge backend.
///
/// ```no_run
/// # use forge_server::ForgeApp;
/// # #[tokio::main] async fn main() -> Result<(), forge_server::ForgeError> {
/// ForgeApp::new("my-app")
///     .auth_from_env()
///     .with_docstore("data")
///     .with_events()
///     .action("echo", |payload, _ctx| async move { Ok(payload) })
///     .serve()
///     .await
/// # }
/// ```
pub struct ForgeApp {
    app: String,
    auth_config: Option<AuthConfig>,
    auth_validator: Option<Arc<dyn TokenValidator>>,
    config_error: Option<ForgeError>,
    events: EventBus,
    events_enabled: bool,
    docstore: Option<DocStore>,
    components_dir: Option<PathBuf>,
    actions: BTreeMap<String, BoxedAction>,
    routes: Vec<(String, MethodRouter<ForgeState>)>,
    frontend: Frontend,
    cors_origins: Option<Vec<String>>,
}

impl ForgeApp {
    /// New builder. Loads `.env` from the working directory (so later
    /// `*_from_env` builder calls see it).
    pub fn new(app: impl Into<String>) -> Self {
        dotenvy::dotenv().ok();
        Self {
            app: app.into(),
            auth_config: None,
            auth_validator: None,
            config_error: None,
            events: EventBus::new(),
            events_enabled: false,
            docstore: None,
            components_dir: None,
            actions: BTreeMap::new(),
            routes: Vec::new(),
            frontend: Frontend::None,
            cors_origins: None,
        }
    }

    fn fail(mut self, e: ForgeError) -> Self {
        if self.config_error.is_none() {
            self.config_error = Some(e);
        }
        self
    }

    /// Enable auth with an explicit config (errors if the secret is < 32 chars).
    pub fn auth(self, config: AuthConfig) -> Self {
        if let Err(e) = config.validate() {
            return self.fail(e);
        }
        Self {
            auth_config: Some(config),
            ..self
        }
    }

    /// Enable auth from `FORGE_JWT_SECRET` / `FORGE_AUTH_USERS` /
    /// `FORGE_JWT_TTL_SECS` / `FORGE_JWT_ISS`. No-op when `FORGE_JWT_SECRET`
    /// is unset (auth-disabled mode); startup fails if it is set but shorter
    /// than 32 characters.
    pub fn auth_from_env(self) -> Self {
        match AuthConfig::from_env() {
            Ok(Some(config)) => self.auth(config),
            Ok(None) => self,
            Err(e) => self.fail(e),
        }
    }

    /// Replace the token validator (e.g. RS256/JWKS). Enables auth. Login
    /// stays available only if an [`AuthConfig`] is also set.
    pub fn auth_validator(mut self, validator: impl TokenValidator + 'static) -> Self {
        self.auth_validator = Some(Arc::new(validator));
        self
    }

    /// Enable the JSON document store in `dir`.
    pub fn with_docstore(mut self, dir: impl Into<PathBuf>) -> Self {
        self.docstore = Some(DocStore::new(dir));
        self
    }

    /// Enable the doc store in `FORGE_DATA_DIR` (default `./data`).
    pub fn with_docstore_from_env(self) -> Self {
        let dir = std::env::var("FORGE_DATA_DIR").unwrap_or_else(|_| "./data".into());
        self.with_docstore(dir)
    }

    /// Mount `/api/events` (SSE) and `/api/ws` (WebSocket).
    pub fn with_events(mut self) -> Self {
        self.events_enabled = true;
        self
    }

    /// Enable component federation from `dir` (must contain `manifest.json`).
    pub fn with_components(mut self, dir: impl Into<PathBuf>) -> Self {
        self.components_dir = Some(dir.into());
        self
    }

    /// Enable components from `FORGE_COMPONENTS_DIR` (default `./components`).
    pub fn with_components_from_env(self) -> Self {
        let dir = std::env::var("FORGE_COMPONENTS_DIR").unwrap_or_else(|_| "./components".into());
        self.with_components(dir)
    }

    /// Register an action, dispatched via `POST /api/actions/{name}`.
    pub fn action<F, Fut>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value, ActionCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, ForgeError>> + Send + 'static,
    {
        self.actions.insert(name.into(), box_action(handler));
        self
    }

    /// Add a custom route (merged before the frontend fallback). Handlers can
    /// extract [`crate::Claims`], `State<ForgeState>` and `State<EventBus>`.
    /// Custom routes are NOT behind the auth middleware — extract
    /// [`crate::Claims`] to require a token.
    pub fn route(mut self, path: impl Into<String>, method_router: MethodRouter<ForgeState>) -> Self {
        self.routes.push((path.into(), method_router));
        self
    }

    /// Serve the frontend from a directory on disk (SPA fallback to `index.html`).
    pub fn frontend_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.frontend = Frontend::Dir(dir.into());
        self
    }

    /// Serve a compile-time embedded frontend (`rust-embed`).
    #[cfg(feature = "embed")]
    pub fn frontend_embedded<A: rust_embed::RustEmbed>(mut self) -> Self {
        self.frontend = Frontend::Embedded(Arc::new(|path| A::get(path).map(|f| f.data)));
        self
    }

    /// Override the CORS origin allowlist (otherwise `FORGE_CORS_ORIGINS` or
    /// the localhost:5173 defaults apply).
    pub fn cors_origins(mut self, origins: Vec<String>) -> Self {
        self.cors_origins = Some(origins);
        self
    }

    /// Handle to the event bus, for publishing from outside handlers
    /// (background tasks, tests).
    pub fn event_bus(&self) -> EventBus {
        self.events.clone()
    }

    /// Build the router (for tests or embedding into a larger app).
    ///
    /// # Panics
    /// Panics on configuration errors (bad `FORGE_JWT_SECRET`, malformed
    /// `FORGE_AUTH_USERS`, ...). Use [`ForgeApp::try_router`] or
    /// [`ForgeApp::serve`] for a `Result`.
    pub fn router(self) -> Router {
        self.try_router().expect("invalid forge-server configuration")
    }

    /// Build the router, surfacing configuration errors.
    pub fn try_router(self) -> Result<Router, ForgeError> {
        if let Some(e) = self.config_error {
            return Err(e);
        }

        let auth_state = match (self.auth_config, self.auth_validator) {
            (None, None) => None,
            (config, validator) => {
                let validator = match (validator, &config) {
                    (Some(v), _) => v,
                    (None, Some(cfg)) => {
                        let mut v = Hs256Validator::new(cfg.secret.clone());
                        if cfg.validate_iss {
                            v = v.with_issuer(cfg.iss.clone());
                        }
                        Arc::new(v) as Arc<dyn TokenValidator>
                    }
                    (None, None) => unreachable!(),
                };
                Some(AuthState { validator, config })
            }
        };

        let cors = build_cors(self.cors_origins)?;

        let state = ForgeState {
            inner: Arc::new(StateInner {
                app: self.app,
                start: Instant::now(),
                auth: auth_state,
                events: self.events,
                docstore: self.docstore,
                actions: self.actions,
                components_dir: self.components_dir,
                frontend: self.frontend,
            }),
        };

        // Protected surface: everything behind the auth middleware. When auth
        // is disabled the middleware stashes anonymous claims and lets all
        // requests through (contract: auth-disabled mode is first-class).
        let mut protected = Router::new()
            .merge(auth::routes::protected_routes())
            .route("/api/actions/{name}", post(crate::actions::run_action));
        if state.inner.docstore.is_some() {
            protected = protected.merge(docstore::routes());
        }
        if self.events_enabled {
            protected = protected
                .route("/api/events", get(events::sse::sse_handler))
                .route("/api/ws", get(events::ws::ws_handler));
        }
        if state.inner.components_dir.is_some() {
            protected = protected.merge(components::routes());
        }
        let protected = protected.route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::extract::auth_middleware,
        ));

        // Open surface: health, login, the frontend, and custom app routes
        // (which opt into auth via the Claims extractor).
        let mut router = Router::new()
            .route("/api/health", get(health::health))
            .merge(auth::routes::open_routes())
            .merge(protected);
        for (path, method_router) in self.routes {
            router = router.route(&path, method_router);
        }
        let router = router.fallback(frontend::fallback).with_state(state);

        Ok(router.layer(cors).layer(TraceLayer::new_for_http()))
    }

    /// Bind `FORGE_HOST:FORGE_PORT` (default `127.0.0.1:8765`) and serve.
    /// Initializes tracing if no global subscriber is set.
    pub async fn serve(self) -> Result<(), ForgeError> {
        use tracing_subscriber::EnvFilter;
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
            )
            .try_init();

        let app = self.app.clone();
        let router = self.try_router()?;

        let host = std::env::var("FORGE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port: u16 = match std::env::var("FORGE_PORT") {
            Ok(raw) => raw
                .parse()
                .map_err(|_| ForgeError::Config(format!("FORGE_PORT is not a port: {raw:?}")))?,
            Err(_) => 8765,
        };

        let listener = tokio::net::TcpListener::bind((host.as_str(), port))
            .await
            .map_err(|e| ForgeError::Config(format!("failed to bind {host}:{port}: {e}")))?;
        tracing::info!(app, %host, port, "forge-server listening");
        axum::serve(listener, router)
            .await
            .map_err(ForgeError::Io)?;
        Ok(())
    }
}

fn build_cors(origins: Option<Vec<String>>) -> Result<CorsLayer, ForgeError> {
    let origins = origins.unwrap_or_else(|| {
        std::env::var("FORGE_CORS_ORIGINS")
            .unwrap_or_else(|_| DEFAULT_CORS_ORIGINS.into())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });
    // Never a wildcard: an explicit origin list, always.
    let mut values = Vec::with_capacity(origins.len());
    for origin in &origins {
        values.push(HeaderValue::from_str(origin).map_err(|_| {
            ForgeError::Config(format!("invalid CORS origin: {origin:?}"))
        })?);
    }
    Ok(CorsLayer::new()
        .allow_origin(values)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]))
}
