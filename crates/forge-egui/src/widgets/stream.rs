//! [`WidgetStream`] over in-process channels. The forge-core session engines
//! pump one bidirectional stream; here both directions are bounded tokio mpsc
//! channels, so the engines run unchanged inside the app — the egui sibling of
//! `forge-tauri/src/widget_stream.rs`.

use std::future::Future;

use forge_core::widgets::{StreamClosed, WidgetMsg, WidgetStream, CHANNEL_CAP};
use tokio::sync::mpsc;

/// The engine's half of one live widget session.
pub(crate) struct EguiWidgetStream {
    out: mpsc::Sender<WidgetMsg>,
    inbox: mpsc::Receiver<WidgetMsg>,
    ctx: egui::Context,
}

impl WidgetStream for EguiWidgetStream {
    async fn recv(&mut self) -> Option<WidgetMsg> {
        // `None` once the widget dropped its `SessionChannels` — the engine
        // reads it as peer-gone and kills its PTY/TCP, exactly like a dropped
        // WebSocket or Tauri channel.
        self.inbox.recv().await
    }

    async fn send(&mut self, msg: WidgetMsg) -> Result<(), StreamClosed> {
        self.out.send(msg).await.map_err(|_| StreamClosed)?;
        // Wake the event loop: frames arrive while the UI is idle.
        self.ctx.request_repaint();
        Ok(())
    }
}

/// The widget's half: frames to (`tx`) and from (`rx`) the engine.
///
/// Owned by widget state on the UI thread — use `try_send`/`try_recv` only,
/// never the blocking or async variants. Both channels are bounded at
/// [`CHANNEL_CAP`], so a full `tx` is backpressure (retry next frame), and a
/// blocked engine is what pauses output when the widget stops draining.
/// Dropping this closes the engine's inbox and thereby ends the session.
pub(crate) struct SessionChannels {
    pub tx: mpsc::Sender<WidgetMsg>,
    pub rx: mpsc::Receiver<WidgetMsg>,
}

/// Open one widget session: build the channel pair and spawn
/// `start(stream)` — e.g. `|s| term::session(s, config)` — onto
/// [`crate::rt::handle`].
pub(crate) fn open_session<F, Fut>(ctx: &egui::Context, start: F) -> SessionChannels
where
    F: FnOnce(EguiWidgetStream) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (tx, inbox) = mpsc::channel(CHANNEL_CAP);
    let (out, rx) = mpsc::channel(CHANNEL_CAP);
    let stream = EguiWidgetStream {
        out,
        inbox,
        ctx: ctx.clone(),
    };
    crate::rt::handle().spawn(start(stream));
    SessionChannels { tx, rx }
}
