//! GET /api/desktop/vnc — VNC viewer WebSocket.
//!
//! The server speaks RFB to the target and streams decoded RGBA rect frames
//! ([`super::proto::encode_rect`]); JSON text frames carry control
//! ([`DesktopClientMsg`] / [`DesktopServerMsg`]). Only the Raw encoding is
//! negotiated (plus DesktopSize), so every update arrives as one RGBA rect.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::net::TcpStream;
use vnc::{ClientKeyEvent, ClientMouseEvent, PixelFormat, VncClient, VncConnector, VncEvent, X11Event};

use super::keymap::keysym;
use super::proto::{encode_rect, DesktopClientMsg, DesktopServerMsg};
use super::DesktopConfig;
use crate::auth::jwt::Claims;
use crate::state::ForgeState;

/// How long to wait for the TCP connect + RFB handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Cadence for draining decoded events and requesting the next incremental
/// update (vnc-rs is poll-driven; its own example uses ~16ms).
const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ForgeState>,
    _claims: Claims,
) -> Response {
    let config = state
        .inner
        .vnc
        .clone()
        .expect("route mounted without vnc config");
    ws.on_upgrade(move |socket| session(socket, config))
}

async fn session(mut socket: WebSocket, config: Arc<DesktopConfig>) {
    // The first frame must be a valid `connect`.
    let (host, port, password) = loop {
        let Some(Ok(msg)) = socket.recv().await else {
            return;
        };
        match msg {
            Message::Text(text) => match serde_json::from_str::<DesktopClientMsg>(&text) {
                Ok(DesktopClientMsg::Connect {
                    host,
                    port,
                    password,
                    // RFB VncAuth has no username.
                    username: _,
                }) => {
                    let Some(host) = host else {
                        return fail(socket, "vnc requires a host").await;
                    };
                    break (host, port.unwrap_or(5900), password.unwrap_or_default());
                }
                _ => return fail(socket, "first frame must be a connect message").await,
            },
            Message::Binary(_) => {
                return fail(socket, "first frame must be a connect message").await
            }
            Message::Close(_) => return,
            _ => {}
        }
    };

    if !config
        .allow_hosts
        .as_ref()
        .is_none_or(|allowed| allowed.iter().any(|a| a == &host))
    {
        return fail(socket, "host is not in the allowed hosts list").await;
    }

    let client = match tokio::time::timeout(CONNECT_TIMEOUT, connect(&host, port, password)).await
    {
        Ok(Ok(client)) => client,
        Ok(Err(message)) => return fail(socket, message).await,
        Err(_) => return fail(socket, format!("vnc connect to {host}:{port} timed out")).await,
    };

    run(&mut socket, &client).await;
    let _ = client.close().await;
    let _ = socket.send(Message::Close(None)).await;
}

async fn connect(host: &str, port: u16, password: String) -> Result<VncClient, String> {
    let tcp = TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("vnc connect to {host}:{port} failed: {e}"))?;
    VncConnector::new(tcp)
        .set_auth_method(async move { Ok(password) })
        .add_encoding(vnc::VncEncoding::DesktopSizePseudo)
        .add_encoding(vnc::VncEncoding::Raw)
        .allow_shared(true)
        .set_pixel_format(PixelFormat::rgba())
        .build()
        .map_err(|e| format!("vnc setup failed: {e}"))?
        .try_start()
        .await
        .map_err(|e| format!("vnc handshake failed: {e}"))?
        .finish()
        .map_err(|e| format!("vnc handshake failed: {e}"))
}

/// Pump the session: browser input → X11 events, decoded VNC events → rect
/// frames. `ready` is sent on the first SetResolution from the handshake.
async fn run(socket: &mut WebSocket, client: &VncClient) {
    let mut ready_sent = false;
    // Last pointer state; wheel/CAD synthesize events at this position.
    let (mut px, mut py, mut pmask) = (0u16, 0u16, 0u8);
    let mut tick = tokio::time::interval(POLL_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break };
                let ok = match msg {
                    Message::Text(text) => match serde_json::from_str::<DesktopClientMsg>(&text) {
                        Ok(msg) => forward_input(client, msg, &mut px, &mut py, &mut pmask).await,
                        Err(e) => {
                            tracing::debug!(error = %e, "ignoring malformed desktop frame");
                            true
                        }
                    },
                    Message::Close(_) => break,
                    _ => true,
                };
                if !ok {
                    let _ = send_ctrl(socket, &DesktopServerMsg::Closed).await;
                    break;
                }
            }
            _ = tick.tick() => {
                loop {
                    match client.poll_event().await {
                        Ok(Some(ev)) => {
                            if !handle_event(socket, ev, &mut ready_sent).await {
                                return;
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            let _ = send_ctrl(socket, &DesktopServerMsg::Closed).await;
                            return;
                        }
                    }
                }
                // Keep an incremental update request outstanding.
                if client.input(X11Event::Refresh).await.is_err() {
                    let _ = send_ctrl(socket, &DesktopServerMsg::Closed).await;
                    return;
                }
            }
        }
    }
}

/// Forward one browser control message to the VNC server. `false` = the
/// connection is gone.
async fn forward_input(
    client: &VncClient,
    msg: DesktopClientMsg,
    px: &mut u16,
    py: &mut u16,
    pmask: &mut u8,
) -> bool {
    match msg {
        DesktopClientMsg::Key { code, key, down } => {
            let Some(sym) = keysym::keysym(&code, key.as_deref()) else {
                return true;
            };
            key_event(client, sym, down).await
        }
        DesktopClientMsg::Mouse { x, y, buttons } => {
            (*px, *py, *pmask) = (x, y, vnc_buttons(buttons));
            pointer_event(client, x, y, *pmask).await
        }
        DesktopClientMsg::Wheel { dx, dy } => {
            // RFB scroll = press+release of buttons 4-7 (masks 0x08..0x40).
            for (delta, neg, pos) in [(dy, 0x08, 0x10), (dx, 0x20, 0x40)] {
                if delta != 0.0 {
                    let btn = if delta < 0.0 { neg } else { pos };
                    if !pointer_event(client, *px, *py, *pmask | btn).await
                        || !pointer_event(client, *px, *py, *pmask).await
                    {
                        return false;
                    }
                }
            }
            true
        }
        DesktopClientMsg::Cad => {
            for sym in keysym::CAD {
                if !key_event(client, sym, true).await {
                    return false;
                }
            }
            for sym in keysym::CAD.into_iter().rev() {
                if !key_event(client, sym, false).await {
                    return false;
                }
            }
            true
        }
        // Already connected; a second connect is a no-op.
        DesktopClientMsg::Connect { .. } => true,
    }
}

/// Forward one decoded VNC event to the browser. `false` = stop the session.
async fn handle_event(socket: &mut WebSocket, ev: VncEvent, ready_sent: &mut bool) -> bool {
    match ev {
        VncEvent::SetResolution(screen) => {
            let msg = if *ready_sent {
                DesktopServerMsg::Resize {
                    width: screen.width,
                    height: screen.height,
                }
            } else {
                *ready_sent = true;
                DesktopServerMsg::Ready {
                    width: screen.width,
                    height: screen.height,
                }
            };
            send_ctrl(socket, &msg).await.is_ok()
        }
        VncEvent::RawImage(rect, data) => {
            let frame = encode_rect(rect.x, rect.y, rect.width, rect.height, &data);
            socket.send(Message::Binary(frame.into())).await.is_ok()
        }
        VncEvent::Error(message) => {
            let _ = send_ctrl(socket, &DesktopServerMsg::Error { message }).await;
            false
        }
        // Raw-only negotiation means no Copy/Jpeg/Cursor events; tolerate
        // anything unexpected rather than tearing the session down.
        other => {
            tracing::debug!(event = ?other, "ignoring vnc event");
            true
        }
    }
}

async fn key_event(client: &VncClient, keysym: u32, down: bool) -> bool {
    client
        .input(X11Event::KeyEvent(ClientKeyEvent {
            keycode: keysym,
            down,
        }))
        .await
        .is_ok()
}

async fn pointer_event(client: &VncClient, x: u16, y: u16, mask: u8) -> bool {
    client
        .input(X11Event::PointerEvent(ClientMouseEvent {
            position_x: x,
            position_y: y,
            bottons: mask,
        }))
        .await
        .is_ok()
}

/// JS `PointerEvent.buttons` (L=1, R=2, M=4) → RFB mask (L=1, M=2, R=4).
fn vnc_buttons(js: u8) -> u8 {
    (js & 1) | ((js & 4) >> 1) | ((js & 2) << 1)
}

async fn send_ctrl(socket: &mut WebSocket, msg: &DesktopServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("DesktopServerMsg serializes");
    socket.send(Message::Text(text.into())).await
}

/// Send an error control frame, then close.
async fn fail(mut socket: WebSocket, message: impl Into<String>) {
    let msg = DesktopServerMsg::Error {
        message: message.into(),
    };
    let _ = send_ctrl(&mut socket, &msg).await;
    let _ = socket.send(Message::Close(None)).await;
}

#[cfg(test)]
mod tests {
    use super::vnc_buttons;

    #[test]
    fn js_buttons_map_to_rfb_mask() {
        assert_eq!(vnc_buttons(0), 0);
        assert_eq!(vnc_buttons(1), 1); // left
        assert_eq!(vnc_buttons(2), 4); // right: JS bit1 → RFB bit2
        assert_eq!(vnc_buttons(4), 2); // middle: JS bit2 → RFB bit1
        assert_eq!(vnc_buttons(7), 7); // all three
    }
}
