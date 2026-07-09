//! forge-tauri — first-class Tauri v2 support for Forge apps.
//!
//! A plugin serving the frozen Forge v1 API contract (docs/api-contract.md)
//! and the streaming widgets (docs/widgets-protocol.md) over pure Tauri IPC:
//! no HTTP server inside the app, same wire shapes, different carrier.
//! `@forge/tauri` is the matching JS client.
//!
//! ```ignore
//! // (ignored in doc-tests: generate_context! needs the app's tauri.conf.json)
//! fn main() {
//!     let forge = forge_tauri::Builder::new("my-app")
//!         .with_docstore_default()
//!         .action("echo", |payload, _ctx| async move { Ok(payload) });
//!     tauri::Builder::default()
//!         .plugin(forge.build())
//!         .run(tauri::generate_context!())
//!         .expect("error while running tauri application");
//! }
//! ```
//!
//! Auth-disabled mode only: over IPC the caller is the app's own webview, so
//! `me` answers with anonymous claims and `login` is the contract's 404.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::time::Instant;

use serde_json::Value;
use tauri::plugin::TauriPlugin;
use tauri::{Emitter, Manager, Runtime};

use forge_core::{box_action, DocStore, EventBus, ForgeError};

mod bridge;
mod commands;
mod state;
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
mod widget_stream;

pub use bridge::ForgeResponse;
pub use forge_core::{ActionCtx, Claims, Event};
pub use state::ForgeState;

#[cfg(any(feature = "vnc", feature = "rdp"))]
pub use forge_core::widgets::DesktopConfig;
#[cfg(feature = "term")]
pub use forge_core::widgets::TermConfig;

/// Where the doc store should live.
enum DocstoreDir {
    /// Explicit directory.
    Path(PathBuf),
    /// `<app_data_dir>/data`, resolved once the app handle exists.
    AppDataDefault,
}

/// Builder for the Forge plugin — mirrors `forge_server::ForgeApp`.
pub struct Builder {
    app: String,
    docstore: Option<DocstoreDir>,
    actions: BTreeMap<String, forge_core::BoxedAction>,
    events: EventBus,
    #[cfg(feature = "term")]
    term: Option<std::sync::Arc<TermConfig>>,
    #[cfg(feature = "vnc")]
    vnc: Option<std::sync::Arc<DesktopConfig>>,
    #[cfg(feature = "rdp")]
    rdp: Option<std::sync::Arc<DesktopConfig>>,
}

impl Builder {
    pub fn new(app: impl Into<String>) -> Self {
        Self {
            app: app.into(),
            docstore: None,
            actions: BTreeMap::new(),
            events: EventBus::new(),
            #[cfg(feature = "term")]
            term: None,
            #[cfg(feature = "vnc")]
            vnc: None,
            #[cfg(feature = "rdp")]
            rdp: None,
        }
    }

    /// Enable the JSON document store in an explicit `dir`.
    pub fn with_docstore(mut self, dir: impl Into<PathBuf>) -> Self {
        self.docstore = Some(DocstoreDir::Path(dir.into()));
        self
    }

    /// Enable the doc store in the platform app-data directory
    /// (`<app_data_dir>/data`, e.g. `~/.local/share/<identifier>/data`).
    pub fn with_docstore_default(mut self) -> Self {
        self.docstore = Some(DocstoreDir::AppDataDefault);
        self
    }

    /// Register an action, dispatched via the `request` command
    /// (`POST /api/actions/{name}` semantics).
    pub fn action<F, Fut>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value, ActionCtx) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, ForgeError>> + Send + 'static,
    {
        self.actions.insert(name.into(), box_action(handler));
        self
    }

    /// Enable the terminal widget with defaults (local shell + ssh allowed).
    ///
    /// SAFETY: this hands the webview a real shell as the app uid — RCE by
    /// design. Trusted dev contexts only.
    #[cfg(feature = "term")]
    pub fn with_term(self) -> Self {
        self.with_term_config(TermConfig::default())
    }

    /// Enable the terminal widget with an explicit [`TermConfig`].
    #[cfg(feature = "term")]
    pub fn with_term_config(mut self, config: TermConfig) -> Self {
        self.term = Some(std::sync::Arc::new(config));
        self
    }

    /// Enable the VNC widget with defaults (any host).
    #[cfg(feature = "vnc")]
    pub fn with_vnc(self) -> Self {
        self.with_vnc_config(DesktopConfig::default())
    }

    /// Enable the VNC widget with an explicit [`DesktopConfig`].
    #[cfg(feature = "vnc")]
    pub fn with_vnc_config(mut self, config: DesktopConfig) -> Self {
        self.vnc = Some(std::sync::Arc::new(config));
        self
    }

    /// Enable the RDP widget with defaults (any host).
    #[cfg(feature = "rdp")]
    pub fn with_rdp(self) -> Self {
        self.with_rdp_config(DesktopConfig::default())
    }

    /// Enable the RDP widget with an explicit [`DesktopConfig`].
    #[cfg(feature = "rdp")]
    pub fn with_rdp_config(mut self, config: DesktopConfig) -> Self {
        self.rdp = Some(std::sync::Arc::new(config));
        self
    }

    /// Handle to the event bus, for publishing from outside actions
    /// (background tasks, tests). Events reach the webview as the single
    /// Tauri event `forge://event` with a `{topic, data}` payload.
    pub fn event_bus(&self) -> EventBus {
        self.events.clone()
    }

    /// Build the Tauri plugin: `.plugin(forge.build())`.
    pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
        let Builder {
            app,
            docstore,
            actions,
            events,
            #[cfg(feature = "term")]
            term,
            #[cfg(feature = "vnc")]
            vnc,
            #[cfg(feature = "rdp")]
            rdp,
        } = self;

        tauri::plugin::Builder::new("forge")
            .invoke_handler(tauri::generate_handler![
                commands::request,
                commands::widget_open,
                commands::widget_send_text,
                commands::widget_send_binary,
                commands::widget_close,
            ])
            .setup(move |app_handle, _api| {
                let docstore = match docstore {
                    Some(DocstoreDir::Path(dir)) => Some(DocStore::new(dir)),
                    Some(DocstoreDir::AppDataDefault) => Some(DocStore::new(
                        app_handle.path().app_data_dir()?.join("data"),
                    )),
                    None => None,
                };

                app_handle.manage(ForgeState {
                    app,
                    start: Instant::now(),
                    docstore,
                    actions,
                    events: events.clone(),
                    #[cfg(feature = "term")]
                    term,
                    #[cfg(feature = "vnc")]
                    vnc,
                    #[cfg(feature = "rdp")]
                    rdp,
                    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
                    sessions: state::SessionMap::default(),
                    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
                    next_session: std::sync::atomic::AtomicU32::new(1),
                });

                // EventBus → webview bridge: one Tauri event, client-side
                // topic filtering (mirrors the SSE/WS fan-out).
                let mut rx = events.subscribe();
                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                let data: Value =
                                    serde_json::from_str(&event.json).unwrap_or(Value::Null);
                                let _ = handle.emit(
                                    "forge://event",
                                    serde_json::json!({ "topic": event.topic, "data": data }),
                                );
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                });

                Ok(())
            })
            .build()
    }
}
