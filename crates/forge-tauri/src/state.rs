//! Plugin state managed on the Tauri app: the pieces of a Forge backend that
//! survive between IPC calls — mirrors forge-server's `StateInner` minus the
//! HTTP-only concerns (auth validators, frontend, custom routes).

use std::collections::BTreeMap;
use std::time::Instant;

use forge_core::{BoxedAction, DocStore, EventBus};

#[cfg(any(feature = "vnc", feature = "rdp"))]
use forge_core::widgets::DesktopConfig;
#[cfg(feature = "term")]
use forge_core::widgets::TermConfig;
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
use forge_core::widgets::WidgetMsg;
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
use std::sync::Arc;

/// Registry of live widget sessions: id → sender feeding the session's
/// inbox. Dropping the sender closes the inbox — the engine sees `None` and
/// winds down (socket-death = session-death, same as the WebSocket path).
#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
pub(crate) type SessionMap =
    Arc<std::sync::Mutex<std::collections::HashMap<u32, tokio::sync::mpsc::Sender<WidgetMsg>>>>;

pub struct ForgeState {
    pub(crate) app: String,
    pub(crate) start: Instant,
    pub(crate) docstore: Option<DocStore>,
    pub(crate) actions: BTreeMap<String, BoxedAction>,
    pub(crate) events: EventBus,
    #[cfg(feature = "term")]
    pub(crate) term: Option<Arc<TermConfig>>,
    #[cfg(feature = "vnc")]
    pub(crate) vnc: Option<Arc<DesktopConfig>>,
    #[cfg(feature = "rdp")]
    pub(crate) rdp: Option<Arc<DesktopConfig>>,
    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
    pub(crate) sessions: SessionMap,
    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
    pub(crate) next_session: std::sync::atomic::AtomicU32,
}

impl ForgeState {
    pub(crate) fn action_names(&self) -> Vec<&str> {
        self.actions.keys().map(String::as_str).collect()
    }
}

#[cfg(test)]
impl ForgeState {
    /// Test constructor that fills the feature-gated fields with defaults.
    pub(crate) fn for_tests(
        docstore: Option<DocStore>,
        actions: BTreeMap<String, BoxedAction>,
    ) -> Self {
        Self {
            app: "forge-tauri-test".into(),
            start: Instant::now(),
            docstore,
            actions,
            events: EventBus::new(),
            #[cfg(feature = "term")]
            term: None,
            #[cfg(feature = "vnc")]
            vnc: None,
            #[cfg(feature = "rdp")]
            rdp: None,
            #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
            sessions: SessionMap::default(),
            #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
            next_session: std::sync::atomic::AtomicU32::new(1),
        }
    }
}
