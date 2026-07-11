//! Tokio runtime ownership for the streaming widgets.
//!
//! Widget sessions (PTY pumps, VNC/RDP protocol tasks) are futures that need
//! a tokio runtime to run on. Two modes:
//!
//! - **Injection** — apps that already own a runtime call [`set_handle`] once
//!   before creating the first widget; sessions spawn onto that runtime.
//! - **Lazy fallback** — otherwise the first widget builds a small 2-worker
//!   multi-thread runtime on demand and leaks it. Leaking is deliberate:
//!   sessions are daemon-like (they live until their widget drops its
//!   channels, i.e. process lifetime from the runtime's point of view), and
//!   there is no shutdown point at which dropping the runtime — which blocks
//!   and kills in-flight sessions — would be correct.

use std::sync::OnceLock;

use tokio::runtime::Handle;

static HANDLE: OnceLock<Handle> = OnceLock::new();

/// [`set_handle`] lost the race: a handle was already installed, either by an
/// earlier call or by the lazy fallback (a widget was created first).
#[derive(Debug, thiserror::Error)]
#[error("widget runtime handle already set")]
pub struct HandleAlreadySet;

/// Set the runtime widget sessions spawn onto. Call once before the first
/// widget is created; apps that already own a runtime should always inject it.
pub fn set_handle(handle: Handle) -> Result<(), HandleAlreadySet> {
    HANDLE.set(handle).map_err(|_| HandleAlreadySet)
}

/// The runtime handle sessions spawn onto, building the lazy fallback runtime
/// on first use if none was injected.
pub(crate) fn handle() -> Handle {
    HANDLE
        .get_or_init(|| {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("forge-egui-widgets")
                .enable_all()
                .build()
                .expect("build widget runtime");
            let handle = rt.handle().clone();
            // Keep the workers alive for the rest of the process (see module
            // docs); the handle stays valid because the runtime never drops.
            std::mem::forget(rt);
            handle
        })
        .clone()
}
