//! GET /api/term — PTY terminal WebSocket (local shell + SSH).
//!
//! Binary frames carry raw tty bytes both ways; JSON text frames carry
//! control ([`TermClientMsg`] / [`TermServerMsg`]). The first client frame
//! must be `start`, which picks local vs ssh.

mod local;
#[cfg(feature = "term-ssh")]
mod ssh;

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;

use super::proto::{TermClientMsg, TermMode, TermServerMsg};
use super::TermConfig;
use crate::auth::jwt::Claims;
use crate::state::ForgeState;

/// How long to wait for the child's exit code after its tty reaches EOF.
const EXIT_WAIT: Duration = Duration::from_secs(5);

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ForgeState>,
    _claims: Claims,
) -> Response {
    let config = state
        .inner
        .term
        .clone()
        .expect("route mounted without term config");
    ws.on_upgrade(move |socket| session(socket, config))
}

async fn session(mut socket: WebSocket, config: Arc<TermConfig>) {
    // The first frame must be a valid `start`.
    let (mode, host, port, username, password, cols, rows) = loop {
        let Some(Ok(msg)) = socket.recv().await else {
            return;
        };
        match msg {
            Message::Text(text) => match serde_json::from_str::<TermClientMsg>(&text) {
                Ok(TermClientMsg::Start {
                    mode,
                    host,
                    port,
                    username,
                    password,
                    cols,
                    rows,
                }) => break (mode, host, port, username, password, cols, rows),
                _ => return fail(socket, "first frame must be a start message").await,
            },
            Message::Binary(_) => {
                return fail(socket, "first frame must be a start message").await
            }
            Message::Close(_) => return,
            // axum answers protocol-level pings itself.
            _ => {}
        }
    };

    match mode {
        TermMode::Local => {
            if !config.allow_local {
                return fail(socket, "local terminal sessions are disabled").await;
            }
            let shell = config
                .shell
                .clone()
                .or_else(|| std::env::var("SHELL").ok())
                .unwrap_or_else(|| "/bin/sh".into());
            let (ctrl, io) = match local::spawn(&shell, cols, rows) {
                Ok(v) => v,
                Err(e) => return fail(socket, e).await,
            };
            if send_ctrl(&mut socket, &TermServerMsg::Ready).await.is_err() {
                return;
            }
            run_local(socket, ctrl, io).await;
        }
        TermMode::Ssh => {
            if !config.allow_ssh {
                return fail(socket, "ssh terminal sessions are disabled").await;
            }
            #[cfg(not(feature = "term-ssh"))]
            {
                let _ = (host, port, username, password);
                fail(
                    socket,
                    "ssh support is not compiled into this server (term-ssh feature)",
                )
                .await;
            }
            #[cfg(feature = "term-ssh")]
            {
                let (Some(host), Some(username), Some(password)) = (host, username, password)
                else {
                    return fail(socket, "ssh requires host, username and password").await;
                };
                if !config
                    .allow_hosts
                    .as_ref()
                    .is_none_or(|allowed| allowed.iter().any(|a| a == &host))
                {
                    return fail(socket, "host is not in the allowed hosts list").await;
                }
                ssh::run(
                    socket,
                    host,
                    port.unwrap_or(22),
                    username,
                    password,
                    cols,
                    rows,
                )
                .await;
            }
        }
    }
}

/// Pump a local PTY: socket binary ⇄ tty bytes, resize control frames, and
/// an `exit` frame once the tty reaches EOF.
async fn run_local(mut socket: WebSocket, ctrl: local::PtyControl, io: local::PtyIo) {
    let local::PtyIo {
        input,
        mut output,
        mut exit,
    } = io;
    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Binary(bytes) => {
                        // Writer gone = child exited; the output arm reports
                        // the exit shortly, so a failed send is fine.
                        let _ = input.send(bytes.to_vec()).await;
                    }
                    Message::Text(text) => match serde_json::from_str::<TermClientMsg>(&text) {
                        Ok(TermClientMsg::Resize { cols, rows }) => ctrl.resize(cols, rows),
                        _ => tracing::debug!("ignoring unexpected term control frame"),
                    },
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            out = output.recv() => match out {
                Some(bytes) => {
                    if socket.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
                None => {
                    // tty EOF: the child is gone (or going) — reap its code.
                    let code = match tokio::time::timeout(EXIT_WAIT, &mut exit).await {
                        Ok(Ok(code)) => code,
                        _ => -1,
                    };
                    let _ = send_ctrl(&mut socket, &TermServerMsg::Exit { code }).await;
                    break;
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
    // ctrl drops here, killing the child if it is still running.
}

async fn send_ctrl(socket: &mut WebSocket, msg: &TermServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("TermServerMsg serializes");
    socket.send(Message::Text(text.into())).await
}

/// Send an error control frame, then close.
async fn fail(mut socket: WebSocket, message: impl Into<String>) {
    let msg = TermServerMsg::Error {
        message: message.into(),
    };
    let _ = send_ctrl(&mut socket, &msg).await;
    let _ = socket.send(Message::Close(None)).await;
}
