//! Shared application state (cheaply cloneable `Arc` handle).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::FromRef;

use crate::actions::BoxedAction;
use crate::auth::AuthState;
use crate::docstore::DocStore;
use crate::events::EventBus;
use crate::frontend::Frontend;

pub(crate) struct StateInner {
    pub app: String,
    pub start: Instant,
    pub auth: Option<AuthState>,
    pub events: EventBus,
    pub docstore: Option<DocStore>,
    pub actions: BTreeMap<String, BoxedAction>,
    pub components_dir: Option<PathBuf>,
    pub frontend: Frontend,
    #[cfg(feature = "term")]
    pub term: Option<Arc<crate::widgets::TermConfig>>,
    #[cfg(feature = "vnc")]
    pub vnc: Option<Arc<crate::widgets::DesktopConfig>>,
    #[cfg(feature = "rdp")]
    pub rdp: Option<Arc<crate::widgets::DesktopConfig>>,
}

/// Router state for a Forge app. Handlers added via
/// [`crate::ForgeApp::route`] can extract `State<ForgeState>`,
/// `State<EventBus>` and [`crate::Claims`] / [`crate::OptionalClaims`].
#[derive(Clone)]
pub struct ForgeState {
    pub(crate) inner: Arc<StateInner>,
}

impl ForgeState {
    /// App name (as passed to [`crate::ForgeApp::new`]).
    pub fn app(&self) -> &str {
        &self.inner.app
    }

    /// Seconds since the state was built.
    pub fn uptime_s(&self) -> f64 {
        self.inner.start.elapsed().as_secs_f64()
    }

    /// Whether auth is enabled.
    pub fn auth_enabled(&self) -> bool {
        self.inner.auth.is_some()
    }

    /// Event bus handle (always available; publishing with no listeners is a no-op).
    pub fn events(&self) -> &EventBus {
        &self.inner.events
    }

    /// Doc store, when configured.
    pub fn docstore(&self) -> Option<&DocStore> {
        self.inner.docstore.as_ref()
    }

    /// Sorted registered action names.
    pub fn action_names(&self) -> Vec<&str> {
        self.inner.actions.keys().map(String::as_str).collect()
    }

    pub(crate) fn auth(&self) -> Option<&AuthState> {
        self.inner.auth.as_ref()
    }
}

impl FromRef<ForgeState> for EventBus {
    fn from_ref(state: &ForgeState) -> Self {
        state.inner.events.clone()
    }
}
