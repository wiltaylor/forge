//! The plugin's invoke surface: one generic `request` command for the data
//! plane plus the four widget session commands.

use serde_json::Value;
use tauri::{command, Runtime, State};

use crate::bridge::{self, ForgeResponse};
use crate::state::ForgeState;

#[cfg(not(any(feature = "term", feature = "vnc", feature = "rdp")))]
const NO_WIDGETS: &str = "widget support is not compiled into this app (forge-tauri features)";

#[command]
pub(crate) async fn request(
    state: State<'_, ForgeState>,
    method: String,
    path: String,
    body: Option<Value>,
) -> Result<ForgeResponse, String> {
    Ok(bridge::handle(&state, &method, &path, body).await)
}

/// Open a widget session: spawns the forge-core engine for `kind`, wired to
/// `on_message` for server→client frames. Returns the session id the
/// `widget_send_*` / `widget_close` commands address.
#[command]
pub(crate) async fn widget_open<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, ForgeState>,
    kind: String,
    on_message: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
) -> Result<u32, String> {
    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
    {
        use std::future::Future;
        use std::pin::Pin;

        use crate::widget_stream::TauriWidgetStream;

        let (tx, inbox) = tokio::sync::mpsc::channel(forge_core::widgets::CHANNEL_CAP);
        let stream = TauriWidgetStream {
            out: on_message,
            inbox,
        };
        let session: Pin<Box<dyn Future<Output = ()> + Send>> = match kind.as_str() {
            #[cfg(feature = "term")]
            "term" => {
                let config = state
                    .term
                    .clone()
                    .ok_or("terminal widget is not enabled (Builder::with_term)")?;
                Box::pin(forge_core::widgets::term::session(stream, config))
            }
            #[cfg(feature = "vnc")]
            "vnc" => {
                let config = state
                    .vnc
                    .clone()
                    .ok_or("vnc widget is not enabled (Builder::with_vnc)")?;
                Box::pin(forge_core::widgets::vnc::session(stream, config))
            }
            #[cfg(feature = "rdp")]
            "rdp" => {
                let config = state
                    .rdp
                    .clone()
                    .ok_or("rdp widget is not enabled (Builder::with_rdp)")?;
                Box::pin(forge_core::widgets::rdp::session(stream, config))
            }
            // Reachable only when a subset of the widget features is compiled.
            #[allow(unreachable_patterns)]
            "term" | "vnc" | "rdp" => {
                return Err(format!(
                    "widget kind {kind:?} is not compiled into this app (forge-tauri features)"
                ))
            }
            _ => return Err(format!("unknown widget kind {kind:?}")),
        };

        let id = state
            .next_session
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        state
            .sessions
            .lock()
            .expect("session registry")
            .insert(id, tx);
        let sessions = state.sessions.clone();
        tauri::async_runtime::spawn(async move {
            session.await;
            sessions.lock().expect("session registry").remove(&id);
        });
        Ok(id)
    }
    #[cfg(not(any(feature = "term", feature = "vnc", feature = "rdp")))]
    {
        let _ = (state, kind, on_message);
        Err(NO_WIDGETS.to_string())
    }
}

/// Forward a control frame (JSON string) into a session.
#[command]
pub(crate) async fn widget_send_text(
    state: State<'_, ForgeState>,
    id: u32,
    text: String,
) -> Result<(), String> {
    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
    {
        send_msg(&state, id, forge_core::widgets::WidgetMsg::Text(text)).await
    }
    #[cfg(not(any(feature = "term", feature = "vnc", feature = "rdp")))]
    {
        let _ = (state, id, text);
        Err(NO_WIDGETS.to_string())
    }
}

/// Forward a payload frame (tty input bytes) into a session. Carried as a
/// JSON number array — term keystrokes are tiny; a raw-body invoke is the
/// future optimization if that ever changes.
#[command]
pub(crate) async fn widget_send_binary(
    state: State<'_, ForgeState>,
    id: u32,
    data: Vec<u8>,
) -> Result<(), String> {
    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
    {
        send_msg(&state, id, forge_core::widgets::WidgetMsg::Binary(data)).await
    }
    #[cfg(not(any(feature = "term", feature = "vnc", feature = "rdp")))]
    {
        let _ = (state, id, data);
        Err(NO_WIDGETS.to_string())
    }
}

/// Close a session (idempotent). Dropping the inbox sender is the close
/// signal: the engine's recv() yields None and it winds down, exactly like a
/// dropped WebSocket.
#[command]
pub(crate) async fn widget_close(state: State<'_, ForgeState>, id: u32) -> Result<(), String> {
    #[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
    state.sessions.lock().expect("session registry").remove(&id);
    #[cfg(not(any(feature = "term", feature = "vnc", feature = "rdp")))]
    let _ = (state, id);
    Ok(())
}

#[cfg(any(feature = "term", feature = "vnc", feature = "rdp"))]
async fn send_msg(
    state: &ForgeState,
    id: u32,
    msg: forge_core::widgets::WidgetMsg,
) -> Result<(), String> {
    let tx = state
        .sessions
        .lock()
        .expect("session registry")
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("no widget session {id}"))?;
    // Bounded send: backpressure on a busy engine, error once it is gone.
    tx.send(msg)
        .await
        .map_err(|_| format!("widget session {id} is closed"))
}
