#![cfg(feature = "term")]
//! Loopback gate for the widget-stream bridge: drive a real forge-core term
//! engine through `EguiWidgetStream` + `open_session` with a headless
//! `egui::Context` (`request_repaint` without a repaint callback is a no-op).
//!
//! The bridge is crate-private (`pub(crate) mod stream`), so this test mounts
//! the same source files with `#[path]` — identical code, compiled into the
//! test crate. No test calls `rt::set_handle`: `open_session` exercises the
//! lazy leaked-runtime path, which is safe across parallel tests precisely
//! because that runtime is process-lifetime.

#[allow(dead_code)]
#[path = "../src/rt.rs"]
mod rt;
#[allow(dead_code)]
#[path = "../src/widgets/stream.rs"]
mod stream;

use std::sync::Arc;
use std::time::Duration;

use forge_core::widgets::proto::{TermClientMsg, TermMode, TermServerMsg};
use forge_core::widgets::{term, TermConfig, WidgetMsg};
use stream::SessionChannels;
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(15);

fn open_term_session() -> SessionChannels {
    let ctx = egui::Context::default();
    let config = Arc::new(TermConfig {
        shell: Some("/bin/sh".into()),
        ..TermConfig::default()
    });
    stream::open_session(&ctx, move |s| term::session(s, config))
}

async fn send_ctrl(session: &SessionChannels, msg: TermClientMsg) {
    let text = serde_json::to_string(&msg).expect("control JSON");
    session
        .tx
        .send(WidgetMsg::Text(text))
        .await
        .expect("engine inbox open");
}

async fn next_frame(session: &mut SessionChannels) -> WidgetMsg {
    timeout(WAIT, session.rx.recv())
        .await
        .expect("timed out waiting for frame")
        .expect("frame channel closed")
}

/// Read frames until a control frame arrives; tty bytes accumulate.
async fn next_ctrl(session: &mut SessionChannels, sink: &mut Vec<u8>) -> TermServerMsg {
    loop {
        match next_frame(session).await {
            WidgetMsg::Text(ctrl) => {
                return serde_json::from_str(&ctrl).expect("server control JSON")
            }
            WidgetMsg::Binary(bytes) => sink.extend_from_slice(&bytes),
            WidgetMsg::Close => panic!("session closed while waiting for control frame"),
        }
    }
}

async fn read_until(session: &mut SessionChannels, buf: &mut Vec<u8>, needle: &str) {
    while !String::from_utf8_lossy(buf).contains(needle) {
        match next_frame(session).await {
            WidgetMsg::Binary(bytes) => buf.extend_from_slice(&bytes),
            WidgetMsg::Text(_) => {}
            WidgetMsg::Close => panic!(
                "session closed before {needle:?}; output: {:?}",
                String::from_utf8_lossy(buf)
            ),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn local_pty_session_over_egui_stream() {
    let mut session = open_term_session();
    send_ctrl(
        &session,
        TermClientMsg::Start {
            mode: TermMode::Local,
            host: None,
            port: None,
            username: None,
            password: None,
            cols: 80,
            rows: 24,
        },
    )
    .await;

    let mut out = Vec::new();
    assert_eq!(
        next_ctrl(&mut session, &mut out).await,
        TermServerMsg::Ready
    );

    // printf assembles the marker so the local echo of the typed command
    // can't satisfy the assertion.
    session
        .tx
        .send(WidgetMsg::Binary(
            b"printf 'forge%s\\n' -egui-ok\n".to_vec(),
        ))
        .await
        .unwrap();
    read_until(&mut session, &mut out, "forge-egui-ok").await;

    // Resize is accepted mid-session (COLUMNS reflects it).
    send_ctrl(
        &session,
        TermClientMsg::Resize {
            cols: 123,
            rows: 24,
        },
    )
    .await;
    session
        .tx
        .send(WidgetMsg::Binary(
            b"stty size | awk '{print \"cols=\" $2}'\n".to_vec(),
        ))
        .await
        .unwrap();
    read_until(&mut session, &mut out, "cols=123").await;

    session
        .tx
        .send(WidgetMsg::Binary(b"exit 7\n".to_vec()))
        .await
        .unwrap();
    assert_eq!(
        next_ctrl(&mut session, &mut out).await,
        TermServerMsg::Exit { code: 7 }
    );

    // The engine closes the stream after exit.
    assert!(matches!(next_frame(&mut session).await, WidgetMsg::Close));
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_input_sender_ends_the_session() {
    let SessionChannels { tx, mut rx } = open_term_session();
    drop(tx);
    // recv() → None → engine returns without frames; the out sender drops.
    assert!(timeout(WAIT, rx.recv()).await.expect("timely").is_none());
}
