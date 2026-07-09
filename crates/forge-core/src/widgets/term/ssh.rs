//! SSH terminal sessions (russh, feature `term-ssh`).
//!
//! Fully async — the russh channel pumps straight into the stream select
//! loop, no bridge threads. Host-key checking is dev-permissive (any server
//! key accepted) — see docs/widgets-protocol.md.

use std::sync::Arc;

use russh::client;
use russh::ChannelMsg;

use super::{fail, send_ctrl};
use crate::widgets::proto::{TermClientMsg, TermServerMsg};
use crate::widgets::{WidgetMsg, WidgetStream};

struct AcceptAllKeys;

impl client::Handler for AcceptAllKeys {
    type Error = russh::Error;

    // Dev-permissive: trust any host key.
    async fn check_server_key(
        &mut self,
        _key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub(super) async fn run<S: WidgetStream>(
    mut stream: S,
    host: String,
    port: u16,
    username: String,
    password: String,
    cols: u16,
    rows: u16,
) {
    let (handle, mut channel) = match connect(&host, port, username, password, cols, rows).await {
        Ok(v) => v,
        Err(message) => return fail(stream, message).await,
    };
    if send_ctrl(&mut stream, &TermServerMsg::Ready).await.is_err() {
        return;
    }

    let mut exit_code: Option<i32> = None;
    loop {
        tokio::select! {
            msg = stream.recv() => {
                let Some(msg) = msg else { break };
                match msg {
                    WidgetMsg::Binary(bytes) => {
                        if channel.data(&bytes[..]).await.is_err() {
                            break;
                        }
                    }
                    WidgetMsg::Text(text) => match serde_json::from_str::<TermClientMsg>(&text) {
                        Ok(TermClientMsg::Resize { cols, rows }) => {
                            let _ = channel
                                .window_change(u32::from(cols), u32::from(rows), 0, 0)
                                .await;
                        }
                        _ => tracing::debug!("ignoring unexpected term control frame"),
                    },
                    WidgetMsg::Close => break,
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data })
                    | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        if stream.send(WidgetMsg::Binary(data.to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        // Data/Eof/Close may still follow; report on Close.
                        exit_code = Some(exit_status as i32);
                    }
                    Some(ChannelMsg::Close) | None => {
                        let msg = TermServerMsg::Exit {
                            code: exit_code.unwrap_or(0),
                        };
                        let _ = send_ctrl(&mut stream, &msg).await;
                        break;
                    }
                    Some(_) => {}
                }
            }
        }
    }

    let _ = handle
        .disconnect(russh::Disconnect::ByApplication, "", "")
        .await;
    let _ = stream.send(WidgetMsg::Close).await;
}

async fn connect(
    host: &str,
    port: u16,
    username: String,
    password: String,
    cols: u16,
    rows: u16,
) -> Result<(client::Handle<AcceptAllKeys>, russh::Channel<client::Msg>), String> {
    let config = Arc::new(client::Config::default());
    let mut handle = client::connect(config, (host, port), AcceptAllKeys)
        .await
        .map_err(|e| format!("ssh connect to {host}:{port} failed: {e}"))?;
    let auth = handle
        .authenticate_password(username, password)
        .await
        .map_err(|e| format!("ssh authentication failed: {e}"))?;
    if !auth.success() {
        return Err("ssh authentication rejected".into());
    }
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("ssh channel open failed: {e}"))?;
    channel
        .request_pty(
            false,
            "xterm-256color",
            u32::from(cols),
            u32::from(rows),
            0,
            0,
            &[],
        )
        .await
        .map_err(|e| format!("ssh pty request failed: {e}"))?;
    channel
        .request_shell(false)
        .await
        .map_err(|e| format!("ssh shell request failed: {e}"))?;
    Ok((handle, channel))
}
