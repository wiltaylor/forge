//! VNC viewer session engine.
//!
//! The backend speaks RFB to the target and streams decoded RGBA rect frames
//! ([`super::proto::encode_rect`]); JSON text frames carry control
//! ([`DesktopClientMsg`] / [`DesktopServerMsg`]). Only the Raw encoding is
//! negotiated (plus DesktopSize), so every update arrives as one RGBA rect.
//! Transport-agnostic: drive it with any [`WidgetStream`].

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use vnc::{
    ClientKeyEvent, ClientMouseEvent, PixelFormat, VncClient, VncConnector, VncEvent, X11Event,
};

use super::keymap::keysym;
use super::proto::{encode_rect, DesktopClientMsg, DesktopServerMsg};
use super::{DesktopConfig, StreamClosed, WidgetMsg, WidgetStream};

/// How long to wait for the TCP connect + RFB handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Cadence for draining decoded events and requesting the next incremental
/// update (vnc-rs is poll-driven; its own example uses ~16ms).
const POLL_INTERVAL: Duration = Duration::from_millis(16);

/// Run one VNC session over `stream`. The first frame must be a valid
/// `connect` message naming the target host.
pub async fn session<S: WidgetStream>(mut stream: S, config: Arc<DesktopConfig>) {
    let Some((host, port, password)) = recv_connect(&mut stream).await else {
        return;
    };
    let Some(host) = host else {
        return fail(&mut stream, "vnc requires a host").await;
    };
    let (port, password) = (port.unwrap_or(5900), password.unwrap_or_default());

    if !config
        .allow_hosts
        .as_ref()
        .is_none_or(|allowed| allowed.iter().any(|a| a == &host))
    {
        return fail(&mut stream, "host is not in the allowed hosts list").await;
    }

    let client = match tokio::time::timeout(CONNECT_TIMEOUT, connect(&host, port, password)).await {
        Ok(Ok(client)) => client,
        Ok(Err(message)) => return fail(&mut stream, message).await,
        Err(_) => {
            return fail(&mut stream, format!("vnc connect to {host}:{port} timed out")).await
        }
    };

    run(&mut stream, &client).await;
    let _ = client.close().await;
    let _ = stream.send(WidgetMsg::Close).await;
}

/// Run one VNC session over a pre-established `transport` (e.g. a QEMU
/// `vnc.sock` UnixStream). The first client frame must still be a `connect`
/// message, but its host/port are ignored — the embedder fixed the target.
/// `password` overrides the frame's password when `Some`.
pub async fn session_over<S, T>(mut stream: S, transport: T, password: Option<String>)
where
    S: WidgetStream,
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
{
    let Some((_host, _port, frame_password)) = recv_connect(&mut stream).await else {
        return;
    };
    let password = password.or(frame_password).unwrap_or_default();

    let client = match tokio::time::timeout(CONNECT_TIMEOUT, handshake(transport, password)).await
    {
        Ok(Ok(client)) => client,
        Ok(Err(message)) => return fail(&mut stream, message).await,
        Err(_) => return fail(&mut stream, "vnc handshake timed out").await,
    };

    run(&mut stream, &client).await;
    let _ = client.close().await;
    let _ = stream.send(WidgetMsg::Close).await;
}

/// Wait for the opening `connect` frame and return its
/// `(host, port, password)`. `None` = the peer closed, or the first frame was
/// invalid (the error frame + close have already been sent).
async fn recv_connect<S: WidgetStream>(
    stream: &mut S,
) -> Option<(Option<String>, Option<u16>, Option<String>)> {
    let msg = stream.recv().await?;
    match msg {
        WidgetMsg::Text(text) => match serde_json::from_str::<DesktopClientMsg>(&text) {
            Ok(DesktopClientMsg::Connect {
                host,
                port,
                password,
                // RFB VncAuth has no username.
                username: _,
            }) => Some((host, port, password)),
            _ => {
                fail(stream, "first frame must be a connect message").await;
                None
            }
        },
        WidgetMsg::Binary(_) => {
            fail(stream, "first frame must be a connect message").await;
            None
        }
        WidgetMsg::Close => None,
    }
}

async fn connect(host: &str, port: u16, password: String) -> Result<VncClient, String> {
    let tcp = TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("vnc connect to {host}:{port} failed: {e}"))?;
    handshake(tcp, password).await
}

async fn handshake<T>(transport: T, password: String) -> Result<VncClient, String>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static,
{
    VncConnector::new(transport)
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

/// Pump the session: client input → X11 events, decoded VNC events → rect
/// frames. `ready` is sent on the first SetResolution from the handshake.
async fn run<S: WidgetStream>(stream: &mut S, client: &VncClient) {
    let mut ready_sent = false;
    // Last pointer state; wheel/CAD synthesize events at this position.
    let (mut px, mut py, mut pmask) = (0u16, 0u16, 0u8);
    let mut tick = tokio::time::interval(POLL_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            msg = stream.recv() => {
                let Some(msg) = msg else { break };
                let ok = match msg {
                    WidgetMsg::Text(text) => match serde_json::from_str::<DesktopClientMsg>(&text) {
                        Ok(msg) => forward_input(client, msg, &mut px, &mut py, &mut pmask).await,
                        Err(e) => {
                            tracing::debug!(error = %e, "ignoring malformed desktop frame");
                            true
                        }
                    },
                    WidgetMsg::Close => break,
                    WidgetMsg::Binary(_) => true,
                };
                if !ok {
                    let _ = send_ctrl(stream, &DesktopServerMsg::Closed).await;
                    break;
                }
            }
            _ = tick.tick() => {
                loop {
                    match client.poll_event().await {
                        Ok(Some(ev)) => {
                            if !handle_event(stream, ev, &mut ready_sent).await {
                                return;
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            let _ = send_ctrl(stream, &DesktopServerMsg::Closed).await;
                            return;
                        }
                    }
                }
                // Keep an incremental update request outstanding.
                if client.input(X11Event::Refresh).await.is_err() {
                    let _ = send_ctrl(stream, &DesktopServerMsg::Closed).await;
                    return;
                }
            }
        }
    }
}

/// Forward one client control message to the VNC server. `false` = the
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

/// Forward one decoded VNC event to the client. `false` = stop the session.
async fn handle_event<S: WidgetStream>(
    stream: &mut S,
    ev: VncEvent,
    ready_sent: &mut bool,
) -> bool {
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
            send_ctrl(stream, &msg).await.is_ok()
        }
        VncEvent::RawImage(rect, data) => {
            let frame = encode_rect(rect.x, rect.y, rect.width, rect.height, &data);
            stream.send(WidgetMsg::Binary(frame)).await.is_ok()
        }
        VncEvent::Error(message) => {
            let _ = send_ctrl(stream, &DesktopServerMsg::Error { message }).await;
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

async fn send_ctrl<S: WidgetStream>(
    stream: &mut S,
    msg: &DesktopServerMsg,
) -> Result<(), StreamClosed> {
    let text = serde_json::to_string(msg).expect("DesktopServerMsg serializes");
    stream.send(WidgetMsg::Text(text)).await
}

/// Send an error control frame, then close.
async fn fail<S: WidgetStream>(stream: &mut S, message: impl Into<String>) {
    let msg = DesktopServerMsg::Error {
        message: message.into(),
    };
    let _ = send_ctrl(stream, &msg).await;
    let _ = stream.send(WidgetMsg::Close).await;
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    #[test]
    fn js_buttons_map_to_rfb_mask() {
        assert_eq!(vnc_buttons(0), 0);
        assert_eq!(vnc_buttons(1), 1); // left
        assert_eq!(vnc_buttons(2), 4); // right: JS bit1 → RFB bit2
        assert_eq!(vnc_buttons(4), 2); // middle: JS bit2 → RFB bit1
        assert_eq!(vnc_buttons(7), 7); // all three
    }

    /// Scripted inbox, captured outbox.
    struct MockStream {
        inbox: VecDeque<WidgetMsg>,
        sent: Vec<WidgetMsg>,
    }

    impl MockStream {
        fn new(frames: impl IntoIterator<Item = WidgetMsg>) -> Self {
            Self {
                inbox: frames.into_iter().collect(),
                sent: Vec::new(),
            }
        }

        fn error_message(&self) -> Option<String> {
            self.sent.iter().find_map(|msg| match msg {
                WidgetMsg::Text(text) => match serde_json::from_str(text) {
                    Ok(DesktopServerMsg::Error { message }) => Some(message),
                    _ => None,
                },
                _ => None,
            })
        }
    }

    /// On `&mut` so the sessions (which take the stream by value) leave the
    /// mock inspectable afterwards.
    impl WidgetStream for &mut MockStream {
        async fn recv(&mut self) -> Option<WidgetMsg> {
            self.inbox.pop_front()
        }

        async fn send(&mut self, msg: WidgetMsg) -> Result<(), StreamClosed> {
            self.sent.push(msg);
            Ok(())
        }
    }

    /// A transport that EOFs immediately, so the RFB handshake fails fast.
    fn dead_transport() -> tokio::io::DuplexStream {
        let (ours, theirs) = tokio::io::duplex(64);
        drop(theirs);
        ours
    }

    #[tokio::test]
    async fn session_over_rejects_a_non_connect_first_frame() {
        let mut stream = MockStream::new([WidgetMsg::Text(
            r#"{"type":"key","code":"KeyA","down":true}"#.into(),
        )]);
        session_over(&mut stream, dead_transport(), None).await;
        assert_eq!(
            stream.error_message().as_deref(),
            Some("first frame must be a connect message")
        );
        assert_eq!(stream.sent.last(), Some(&WidgetMsg::Close));
    }

    #[tokio::test]
    async fn session_over_does_not_require_a_host() {
        let mut stream = MockStream::new([WidgetMsg::Text(r#"{"type":"connect"}"#.into())]);
        session_over(&mut stream, dead_transport(), None).await;
        // The host-less connect frame is accepted; the failure comes from the
        // dead transport's handshake, not connect-frame validation.
        let message = stream.error_message().expect("handshake error frame");
        assert!(message.starts_with("vnc handshake failed"), "{message}");
        assert_eq!(stream.sent.last(), Some(&WidgetMsg::Close));
    }
}
