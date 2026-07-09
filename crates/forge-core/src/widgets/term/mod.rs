//! PTY terminal session engine (local shell + SSH).
//!
//! Binary frames carry raw tty bytes both ways; JSON text frames carry
//! control ([`TermClientMsg`] / [`TermServerMsg`]). The first client frame
//! must be `start`, which picks local vs ssh. Transport-agnostic: drive it
//! with any [`WidgetStream`].

mod local;
#[cfg(feature = "term-ssh")]
mod ssh;

use std::sync::Arc;
use std::time::Duration;

use super::proto::{TermClientMsg, TermMode, TermServerMsg};
use super::{TermConfig, WidgetMsg, WidgetStream};

/// How long to wait for the child's exit code after its tty reaches EOF.
const EXIT_WAIT: Duration = Duration::from_secs(5);

/// Run one terminal session over `stream`. The first frame must be a valid
/// `start` message.
pub async fn session<S: WidgetStream>(mut stream: S, config: Arc<TermConfig>) {
    let (mode, host, port, username, password, cols, rows) = loop {
        let Some(msg) = stream.recv().await else {
            return;
        };
        match msg {
            WidgetMsg::Text(text) => match serde_json::from_str::<TermClientMsg>(&text) {
                Ok(TermClientMsg::Start {
                    mode,
                    host,
                    port,
                    username,
                    password,
                    cols,
                    rows,
                }) => break (mode, host, port, username, password, cols, rows),
                _ => return fail(stream, "first frame must be a start message").await,
            },
            WidgetMsg::Binary(_) => {
                return fail(stream, "first frame must be a start message").await
            }
            WidgetMsg::Close => return,
        }
    };

    match mode {
        TermMode::Local => {
            if !config.allow_local {
                return fail(stream, "local terminal sessions are disabled").await;
            }
            let shell = config
                .shell
                .clone()
                .or_else(|| std::env::var("SHELL").ok())
                .unwrap_or_else(|| "/bin/sh".into());
            let (ctrl, io) = match local::spawn(&shell, cols, rows) {
                Ok(v) => v,
                Err(e) => return fail(stream, e).await,
            };
            if send_ctrl(&mut stream, &TermServerMsg::Ready).await.is_err() {
                return;
            }
            run_local(stream, ctrl, io).await;
        }
        TermMode::Ssh => {
            if !config.allow_ssh {
                return fail(stream, "ssh terminal sessions are disabled").await;
            }
            #[cfg(not(feature = "term-ssh"))]
            {
                let _ = (host, port, username, password);
                fail(
                    stream,
                    "ssh support is not compiled into this server (term-ssh feature)",
                )
                .await;
            }
            #[cfg(feature = "term-ssh")]
            {
                let (Some(host), Some(username), Some(password)) = (host, username, password)
                else {
                    return fail(stream, "ssh requires host, username and password").await;
                };
                if !config
                    .allow_hosts
                    .as_ref()
                    .is_none_or(|allowed| allowed.iter().any(|a| a == &host))
                {
                    return fail(stream, "host is not in the allowed hosts list").await;
                }
                ssh::run(
                    stream,
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

/// Pump a local PTY: stream binary ⇄ tty bytes, resize control frames, and
/// an `exit` frame once the tty reaches EOF.
async fn run_local<S: WidgetStream>(mut stream: S, ctrl: local::PtyControl, io: local::PtyIo) {
    let local::PtyIo {
        input,
        mut output,
        mut exit,
    } = io;
    loop {
        tokio::select! {
            msg = stream.recv() => {
                let Some(msg) = msg else { break };
                match msg {
                    WidgetMsg::Binary(bytes) => {
                        // Writer gone = child exited; the output arm reports
                        // the exit shortly, so a failed send is fine.
                        let _ = input.send(bytes).await;
                    }
                    WidgetMsg::Text(text) => match serde_json::from_str::<TermClientMsg>(&text) {
                        Ok(TermClientMsg::Resize { cols, rows }) => ctrl.resize(cols, rows),
                        _ => tracing::debug!("ignoring unexpected term control frame"),
                    },
                    WidgetMsg::Close => break,
                }
            }
            out = output.recv() => match out {
                Some(bytes) => {
                    if stream.send(WidgetMsg::Binary(bytes)).await.is_err() {
                        break;
                    }
                }
                None => {
                    // tty EOF: the child is gone (or going) — reap its code.
                    let code = match tokio::time::timeout(EXIT_WAIT, &mut exit).await {
                        Ok(Ok(code)) => code,
                        _ => -1,
                    };
                    let _ = send_ctrl(&mut stream, &TermServerMsg::Exit { code }).await;
                    break;
                }
            }
        }
    }
    let _ = stream.send(WidgetMsg::Close).await;
    // ctrl drops here, killing the child if it is still running.
}

async fn send_ctrl<S: WidgetStream>(
    stream: &mut S,
    msg: &TermServerMsg,
) -> Result<(), super::StreamClosed> {
    let text = serde_json::to_string(msg).expect("TermServerMsg serializes");
    stream.send(WidgetMsg::Text(text)).await
}

/// Send an error control frame, then close.
async fn fail<S: WidgetStream>(mut stream: S, message: impl Into<String>) {
    let msg = TermServerMsg::Error {
        message: message.into(),
    };
    let _ = send_ctrl(&mut stream, &msg).await;
    let _ = stream.send(WidgetMsg::Close).await;
}
