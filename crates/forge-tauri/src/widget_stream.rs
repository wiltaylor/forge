//! [`WidgetStream`] over Tauri IPC. Server→client frames ride the session's
//! `ipc::Channel`; client→server frames arrive through the `widget_send_*`
//! commands into an mpsc inbox. Together the pair is one bidirectional
//! stream, so the forge-core engines run unchanged.

use forge_core::widgets::{StreamClosed, WidgetMsg, WidgetStream};
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::sync::mpsc;

/// One live widget session's transport half.
pub(crate) struct TauriWidgetStream {
    pub(crate) out: Channel<InvokeResponseBody>,
    pub(crate) inbox: mpsc::Receiver<WidgetMsg>,
}

impl WidgetStream for TauriWidgetStream {
    async fn recv(&mut self) -> Option<WidgetMsg> {
        // `None` once every registered sender is dropped (widget_close, or
        // plugin teardown) — the engine reads it as peer-gone.
        self.inbox.recv().await
    }

    async fn send(&mut self, msg: WidgetMsg) -> Result<(), StreamClosed> {
        // Preserve the protocol's frame discriminator across the channel:
        // control JSON goes as a JSON string (JS receives a string), payload
        // bytes go raw (JS receives an ArrayBuffer), close goes as JSON
        // `null` — unambiguous, since control frames are always objects.
        let frame = match msg {
            WidgetMsg::Text(text) => {
                InvokeResponseBody::Json(serde_json::to_string(&text).map_err(|_| StreamClosed)?)
            }
            WidgetMsg::Binary(bytes) => InvokeResponseBody::Raw(bytes),
            WidgetMsg::Close => InvokeResponseBody::Json("null".to_string()),
        };
        self.out.send(frame).map_err(|_| StreamClosed)
    }
}

// The plan's loopback gate: drive a real forge-core engine through
// TauriWidgetStream with a closure-backed Channel — no webview, no window.
#[cfg(all(test, feature = "term"))]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use forge_core::widgets::{term, TermConfig, WidgetMsg};
    use serde_json::{json, Value};
    use tokio::time::timeout;

    use super::*;

    const WAIT: Duration = Duration::from_secs(10);

    /// What the JS side would see: a string (control), bytes (payload), or
    /// close (JSON null).
    #[derive(Debug)]
    enum JsFrame {
        Text(Value),
        Binary(Vec<u8>),
        Close,
    }

    fn decode(frame: InvokeResponseBody) -> JsFrame {
        match frame {
            InvokeResponseBody::Raw(bytes) => JsFrame::Binary(bytes),
            InvokeResponseBody::Json(raw) => {
                let value: Value = serde_json::from_str(&raw).expect("channel JSON");
                match value {
                    Value::Null => JsFrame::Close,
                    Value::String(ctrl) => {
                        JsFrame::Text(serde_json::from_str(&ctrl).expect("control JSON"))
                    }
                    other => panic!("unexpected channel frame: {other}"),
                }
            }
        }
    }

    struct Session {
        input: mpsc::Sender<WidgetMsg>,
        frames: mpsc::UnboundedReceiver<JsFrame>,
    }

    fn spawn_term_session() -> Session {
        let (frame_tx, frames) = mpsc::unbounded_channel();
        let channel = Channel::new(move |frame| {
            let _ = frame_tx.send(decode(frame));
            Ok(())
        });
        let (input, inbox) = mpsc::channel(forge_core::widgets::CHANNEL_CAP);
        let stream = TauriWidgetStream {
            out: channel,
            inbox,
        };
        let config = Arc::new(TermConfig {
            shell: Some("/bin/sh".into()),
            ..TermConfig::default()
        });
        tokio::spawn(term::session(stream, config));
        Session { input, frames }
    }

    async fn next_frame(session: &mut Session) -> JsFrame {
        timeout(WAIT, session.frames.recv())
            .await
            .expect("timed out waiting for frame")
            .expect("frame channel closed")
    }

    /// Read frames until a control frame arrives; tty bytes accumulate.
    async fn next_ctrl(session: &mut Session, sink: &mut Vec<u8>) -> Value {
        loop {
            match next_frame(session).await {
                JsFrame::Text(ctrl) => return ctrl,
                JsFrame::Binary(bytes) => sink.extend_from_slice(&bytes),
                JsFrame::Close => panic!("session closed while waiting for control frame"),
            }
        }
    }

    async fn read_until(session: &mut Session, buf: &mut Vec<u8>, needle: &str) {
        while !String::from_utf8_lossy(buf).contains(needle) {
            match next_frame(session).await {
                JsFrame::Binary(bytes) => buf.extend_from_slice(&bytes),
                JsFrame::Text(_) => {}
                JsFrame::Close => panic!(
                    "session closed before {needle:?}; output: {:?}",
                    String::from_utf8_lossy(buf)
                ),
            }
        }
    }

    #[tokio::test]
    async fn local_pty_session_over_ipc_stream() {
        let mut session = spawn_term_session();
        session
            .input
            .send(WidgetMsg::Text(
                json!({"type":"start","mode":"local","cols":80,"rows":24}).to_string(),
            ))
            .await
            .unwrap();

        let mut out = Vec::new();
        let ready = next_ctrl(&mut session, &mut out).await;
        assert_eq!(ready["type"], "ready");

        // printf assembles the marker so the local echo of the typed command
        // can't satisfy the assertion.
        session
            .input
            .send(WidgetMsg::Binary(b"printf 'forge%s\\n' -ipc-ok\n".to_vec()))
            .await
            .unwrap();
        read_until(&mut session, &mut out, "forge-ipc-ok").await;

        // Resize is accepted mid-session (COLUMNS reflects it).
        session
            .input
            .send(WidgetMsg::Text(
                json!({"type":"resize","cols":123,"rows":24}).to_string(),
            ))
            .await
            .unwrap();
        session
            .input
            .send(WidgetMsg::Binary(
                b"stty size | awk '{print \"cols=\" $2}'\n".to_vec(),
            ))
            .await
            .unwrap();
        read_until(&mut session, &mut out, "cols=123").await;

        session
            .input
            .send(WidgetMsg::Binary(b"exit 7\n".to_vec()))
            .await
            .unwrap();
        let exit = next_ctrl(&mut session, &mut out).await;
        assert_eq!(exit["type"], "exit");
        assert_eq!(exit["code"], 7);

        // The engine closes the stream after exit.
        assert!(matches!(next_frame(&mut session).await, JsFrame::Close));
    }

    #[tokio::test]
    async fn dropping_the_input_sender_ends_the_session() {
        let session = spawn_term_session();
        let Session { input, mut frames } = session;
        drop(input);
        // recv() → None → engine returns without frames; the channel closes.
        assert!(timeout(WAIT, frames.recv())
            .await
            .expect("timely")
            .is_none());
    }
}
