//! RDP viewer session engine (IronRDP).
//!
//! The backend runs a full IronRDP client — TLS upgrade (dev-permissive
//! verifier, see docs/widgets-protocol.md), CredSSP/NLA, bitmap decode into
//! an RGBA [`DecodedImage`] — and streams dirty rects as
//! [`super::proto::encode_rect`] frames. Client input becomes FastPath
//! input PDUs via `ironrdp-input`'s state [`Database`].
//! Transport-agnostic: drive it with any [`WidgetStream`].

use std::sync::Arc;
use std::time::Duration;

use ironrdp::connector::{self, Credentials};
use ironrdp::graphics::image_processing::PixelFormat;
use ironrdp::input::{Database, MouseButton, MousePosition, Operation, Scancode, WheelRotations};
use ironrdp::pdu::gcc::KeyboardType;
use ironrdp::pdu::geometry::{InclusiveRectangle, Rectangle as _};
use ironrdp::pdu::rdp::capability_sets::MajorPlatformType;
use ironrdp::pdu::rdp::client_info::{PerformanceFlags, TimezoneInfo};
use ironrdp::session::image::DecodedImage;
use ironrdp::session::{ActiveStage, ActiveStageOutput};
use ironrdp_tokio::{FramedWrite as _, TokioFramed};
use tokio::net::TcpStream;

use super::keymap::scancode;
use super::proto::{encode_rect, DesktopClientMsg, DesktopServerMsg};
use super::{DesktopConfig, StreamClosed, WidgetMsg, WidgetStream};

/// How long to wait for TCP + TLS + CredSSP + capability exchange.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Desktop size requested from the server (wire protocol v1 carries no
/// client hint; the server may pick something else — `ready` reports it).
const DESKTOP_SIZE: connector::DesktopSize = connector::DesktopSize {
    width: 1280,
    height: 800,
};

type RdpFramed = TokioFramed<ironrdp_tls::TlsStream<TcpStream>>;

/// Run one RDP session over `stream`. The first frame must be a valid
/// `connect` message with full credentials.
pub async fn session<S: WidgetStream>(mut stream: S, config: Arc<DesktopConfig>) {
    let (host, port, username, password) = loop {
        let Some(msg) = stream.recv().await else {
            return;
        };
        match msg {
            WidgetMsg::Text(text) => match serde_json::from_str::<DesktopClientMsg>(&text) {
                Ok(DesktopClientMsg::Connect {
                    host,
                    port,
                    username,
                    password,
                }) => {
                    let (Some(host), Some(username), Some(password)) = (host, username, password)
                    else {
                        return fail(stream, "rdp requires host, username and password").await;
                    };
                    break (host, port.unwrap_or(3389), username, password);
                }
                _ => return fail(stream, "first frame must be a connect message").await,
            },
            WidgetMsg::Binary(_) => {
                return fail(stream, "first frame must be a connect message").await
            }
            WidgetMsg::Close => return,
        }
    };

    if !config
        .allow_hosts
        .as_ref()
        .is_none_or(|allowed| allowed.iter().any(|a| a == &host))
    {
        return fail(stream, "host is not in the allowed hosts list").await;
    }

    let (connection_result, framed) =
        match tokio::time::timeout(CONNECT_TIMEOUT, connect(&host, port, username, password)).await
        {
            Ok(Ok(v)) => v,
            Ok(Err(message)) => return fail(stream, message).await,
            Err(_) => return fail(stream, format!("rdp connect to {host}:{port} timed out")).await,
        };

    run(&mut stream, connection_result, framed).await;
    let _ = stream.send(WidgetMsg::Close).await;
}

/// CredSSP network client stub: KDC round-trips (Kerberos) are unsupported;
/// NTLM — the norm for username/password NLA — never needs one.
struct NoKerberos;

impl ironrdp_tokio::NetworkClient for NoKerberos {
    async fn send(
        &mut self,
        _request: &connector::sspi::generator::NetworkRequest,
    ) -> connector::ConnectorResult<Vec<u8>> {
        Err(connector::general_err!(
            "kerberos KDC requests are not supported"
        ))
    }
}

async fn connect(
    host: &str,
    port: u16,
    username: String,
    password: String,
) -> Result<(connector::ConnectionResult, RdpFramed), String> {
    let tcp = TcpStream::connect((host, port))
        .await
        .map_err(|e| format!("rdp connect to {host}:{port} failed: {e}"))?;
    let client_addr = tcp.local_addr().map_err(|e| format!("local addr: {e}"))?;

    let mut framed = TokioFramed::new(tcp);
    let mut rdp_connector =
        connector::ClientConnector::new(build_config(username, password), client_addr);

    let should_upgrade = ironrdp_tokio::connect_begin(&mut framed, &mut rdp_connector)
        .await
        .map_err(|e| format!("rdp negotiation failed: {e}"))?;

    let initial_stream = framed.into_inner_no_leftover();
    let (tls_stream, cert) = ironrdp_tls::upgrade(initial_stream, host)
        .await
        .map_err(|e| format!("rdp tls upgrade failed: {e}"))?;
    let server_public_key = ironrdp_tls::extract_tls_server_public_key(&cert)
        .ok_or("server certificate has no public key")?
        .to_vec();

    let upgraded = ironrdp_tokio::mark_as_upgraded(should_upgrade, &mut rdp_connector);
    let mut framed = TokioFramed::new(tls_stream);
    let connection_result = ironrdp_tokio::connect_finalize(
        upgraded,
        rdp_connector,
        &mut framed,
        &mut NoKerberos,
        connector::ServerName::new(host),
        server_public_key,
        None,
    )
    .await
    .map_err(|e| format!("rdp connection failed: {e}"))?;

    Ok((connection_result, framed))
}

fn build_config(username: String, password: String) -> connector::Config {
    // "DOMAIN\user" splits into an explicit domain.
    let (domain, username) = match username.split_once('\\') {
        Some((d, u)) => (Some(d.to_owned()), u.to_owned()),
        None => (None, username),
    };
    connector::Config {
        credentials: Credentials::UsernamePassword { username, password },
        domain,
        enable_tls: true,
        enable_credssp: true,
        keyboard_type: KeyboardType::IbmEnhanced,
        keyboard_subtype: 0,
        keyboard_layout: 0,
        keyboard_functional_keys_count: 12,
        ime_file_name: String::new(),
        dig_product_id: String::new(),
        desktop_size: DESKTOP_SIZE,
        bitmap: None,
        client_build: 0,
        client_name: "forge-desktop".to_owned(),
        client_dir: "C:\\Windows\\System32\\mstscax.dll".to_owned(),
        platform: MajorPlatformType::UNIX,
        enable_server_pointer: false,
        request_data: None,
        autologon: false,
        enable_audio_playback: false,
        compression_type: None,
        pointer_software_rendering: true,
        multitransport_flags: None,
        performance_flags: PerformanceFlags::default(),
        desktop_scale_factor: 0,
        hardware_id: None,
        license_cache: None,
        timezone_info: TimezoneInfo::default(),
        alternate_shell: String::new(),
        work_dir: String::new(),
    }
}

/// Pump the session: server PDUs → decoded image → dirty rect frames,
/// client input → FastPath input PDUs.
async fn run<S: WidgetStream>(
    stream: &mut S,
    connection_result: connector::ConnectionResult,
    mut framed: RdpFramed,
) {
    let mut image = DecodedImage::new(
        PixelFormat::RgbA32,
        connection_result.desktop_size.width,
        connection_result.desktop_size.height,
    );
    let mut stage = ActiveStage::new(connection_result);
    let mut input_db = Database::new();
    // Last seen JS PointerEvent.buttons bitmask, diffed into transitions.
    let mut js_buttons: u8 = 0;

    let ready = DesktopServerMsg::Ready {
        width: image.width(),
        height: image.height(),
    };
    if send_ctrl(stream, &ready).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            pdu = framed.read_pdu() => {
                let (action, payload) = match pdu {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::debug!(error = %e, "rdp stream ended");
                        let _ = send_ctrl(stream, &DesktopServerMsg::Closed).await;
                        return;
                    }
                };
                let outputs = match stage.process(&mut image, action, &payload) {
                    Ok(outputs) => outputs,
                    Err(e) => {
                        let msg = DesktopServerMsg::Error { message: format!("rdp session error: {e}") };
                        let _ = send_ctrl(stream, &msg).await;
                        return;
                    }
                };
                if !forward_outputs(stream, &mut framed, &image, outputs).await {
                    return;
                }
            }
            msg = stream.recv() => {
                let Some(msg) = msg else { break };
                match msg {
                    WidgetMsg::Text(text) => {
                        let Ok(msg) = serde_json::from_str::<DesktopClientMsg>(&text) else {
                            continue;
                        };
                        let ops = to_operations(msg, &mut js_buttons);
                        if ops.is_empty() {
                            continue;
                        }
                        let events = input_db.apply(ops);
                        let outputs = match stage.process_fastpath_input(&mut image, &events) {
                            Ok(outputs) => outputs,
                            Err(e) => {
                                let msg = DesktopServerMsg::Error { message: format!("rdp input error: {e}") };
                                let _ = send_ctrl(stream, &msg).await;
                                return;
                            }
                        };
                        if !forward_outputs(stream, &mut framed, &image, outputs).await {
                            return;
                        }
                    }
                    WidgetMsg::Close => break,
                    WidgetMsg::Binary(_) => {}
                }
            }
        }
    }

    // Client went away: attempt a graceful RDP shutdown.
    if let Ok(outputs) = stage.graceful_shutdown() {
        for out in outputs {
            if let ActiveStageOutput::ResponseFrame(frame) = out {
                let _ = framed.write_all(&frame).await;
            }
        }
    }
}

/// Forward one batch of ActiveStage outputs. `false` = session over.
async fn forward_outputs<S: WidgetStream>(
    stream: &mut S,
    framed: &mut RdpFramed,
    image: &DecodedImage,
    outputs: Vec<ActiveStageOutput>,
) -> bool {
    for out in outputs {
        match out {
            ActiveStageOutput::ResponseFrame(frame) => {
                if framed.write_all(&frame).await.is_err() {
                    let _ = send_ctrl(stream, &DesktopServerMsg::Closed).await;
                    return false;
                }
            }
            ActiveStageOutput::GraphicsUpdate(rect) => {
                let frame = rect_frame(image, &rect);
                if stream.send(WidgetMsg::Binary(frame)).await.is_err() {
                    return false;
                }
            }
            ActiveStageOutput::Terminate(reason) => {
                tracing::debug!(%reason, "rdp server terminated the session");
                let _ = send_ctrl(stream, &DesktopServerMsg::Closed).await;
                return false;
            }
            ActiveStageOutput::DeactivateAll(_) => {
                let msg = DesktopServerMsg::Error {
                    message: "server requested reactivation (e.g. resolution change) — \
                              not supported, reconnect"
                        .into(),
                };
                let _ = send_ctrl(stream, &msg).await;
                return false;
            }
            _ => {}
        }
    }
    true
}

/// Pack a dirty rect out of the decoded image into a wire rect frame.
fn rect_frame(image: &DecodedImage, rect: &InclusiveRectangle) -> Vec<u8> {
    let (w, h) = (rect.width(), rect.height());
    let stride = image.stride();
    let bpp = image.bytes_per_pixel();
    let data = image.data();
    let mut rgba = Vec::with_capacity(usize::from(w) * usize::from(h) * bpp);
    for row in rect.top..=rect.bottom {
        let start = usize::from(row) * stride + usize::from(rect.left) * bpp;
        rgba.extend_from_slice(&data[start..start + usize::from(w) * bpp]);
    }
    encode_rect(rect.left, rect.top, w, h, &rgba)
}

/// JS `PointerEvent.buttons` bit → RDP mouse button, for mask diffing.
const JS_BUTTON_BITS: [(u8, MouseButton); 5] = [
    (1, MouseButton::Left),
    (2, MouseButton::Right),
    (4, MouseButton::Middle),
    (8, MouseButton::X1),
    (16, MouseButton::X2),
];

/// One wheel notch in RDP rotation units (mstsc sends ±120).
const WHEEL_NOTCH: i16 = 120;

/// Translate one client control message into input-state operations.
fn to_operations(msg: DesktopClientMsg, js_buttons: &mut u8) -> Vec<Operation> {
    let mut ops = Vec::new();
    match msg {
        DesktopClientMsg::Key { code, key, down } => {
            if let Some((sc, extended)) = scancode::scancode(&code) {
                #[allow(clippy::cast_possible_truncation)] // table values are all <= 0x58
                let sc = Scancode::from_u8(extended, sc as u8);
                ops.push(if down {
                    Operation::KeyPressed(sc)
                } else {
                    Operation::KeyReleased(sc)
                });
            } else if let Some(c) = single_char(key.as_deref()) {
                // Unmapped physical key producing a character (non-US
                // layouts): fall back to Unicode input events.
                ops.push(if down {
                    Operation::UnicodeKeyPressed(c)
                } else {
                    Operation::UnicodeKeyReleased(c)
                });
            }
        }
        DesktopClientMsg::Mouse { x, y, buttons } => {
            ops.push(Operation::MouseMove(MousePosition { x, y }));
            for (bit, button) in JS_BUTTON_BITS {
                let (was, is) = (*js_buttons & bit != 0, buttons & bit != 0);
                if is && !was {
                    ops.push(Operation::MouseButtonPressed(button));
                } else if !is && was {
                    ops.push(Operation::MouseButtonReleased(button));
                }
            }
            *js_buttons = buttons;
        }
        DesktopClientMsg::Wheel { dx, dy } => {
            // Browser deltaY > 0 = scroll down = negative RDP rotation.
            if dy != 0.0 {
                ops.push(Operation::WheelRotations(WheelRotations {
                    is_vertical: true,
                    rotation_units: if dy < 0.0 { WHEEL_NOTCH } else { -WHEEL_NOTCH },
                }));
            }
            if dx != 0.0 {
                ops.push(Operation::WheelRotations(WheelRotations {
                    is_vertical: false,
                    rotation_units: if dx < 0.0 { -WHEEL_NOTCH } else { WHEEL_NOTCH },
                }));
            }
        }
        DesktopClientMsg::Cad => {
            for (sc, ext) in scancode::CAD {
                #[allow(clippy::cast_possible_truncation)]
                ops.push(Operation::KeyPressed(Scancode::from_u8(ext, sc as u8)));
            }
            for (sc, ext) in scancode::CAD.into_iter().rev() {
                #[allow(clippy::cast_possible_truncation)]
                ops.push(Operation::KeyReleased(Scancode::from_u8(ext, sc as u8)));
            }
        }
        // Already connected; a second connect is a no-op.
        DesktopClientMsg::Connect { .. } => {}
    }
    ops
}

fn single_char(key: Option<&str>) -> Option<char> {
    let mut chars = key?.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}

async fn send_ctrl<S: WidgetStream>(
    stream: &mut S,
    msg: &DesktopServerMsg,
) -> Result<(), StreamClosed> {
    let text = serde_json::to_string(msg).expect("DesktopServerMsg serializes");
    stream.send(WidgetMsg::Text(text)).await
}

/// Send an error control frame, then close.
async fn fail<S: WidgetStream>(mut stream: S, message: impl Into<String>) {
    let msg = DesktopServerMsg::Error {
        message: message.into(),
    };
    let _ = send_ctrl(&mut stream, &msg).await;
    let _ = stream.send(WidgetMsg::Close).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_mask_diffs_into_transitions() {
        let mut state = 0u8;
        let ops = to_operations(
            DesktopClientMsg::Mouse {
                x: 5,
                y: 6,
                buttons: 1,
            },
            &mut state,
        );
        assert_eq!(ops.len(), 2); // move + left press
        assert!(matches!(
            ops[1],
            Operation::MouseButtonPressed(MouseButton::Left)
        ));

        // Left stays down, right joins: only right press emitted.
        let ops = to_operations(
            DesktopClientMsg::Mouse {
                x: 5,
                y: 6,
                buttons: 3,
            },
            &mut state,
        );
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            ops[1],
            Operation::MouseButtonPressed(MouseButton::Right)
        ));

        // All released.
        let ops = to_operations(
            DesktopClientMsg::Mouse {
                x: 5,
                y: 6,
                buttons: 0,
            },
            &mut state,
        );
        assert_eq!(ops.len(), 3);
        assert!(matches!(
            ops[1],
            Operation::MouseButtonReleased(MouseButton::Left)
        ));
        assert!(matches!(
            ops[2],
            Operation::MouseButtonReleased(MouseButton::Right)
        ));
    }

    #[test]
    fn keys_map_to_scancodes_with_unicode_fallback() {
        let mut state = 0u8;
        let ops = to_operations(
            DesktopClientMsg::Key {
                code: "KeyA".into(),
                key: Some("a".into()),
                down: true,
            },
            &mut state,
        );
        assert!(matches!(ops[0], Operation::KeyPressed(_)));

        let ops = to_operations(
            DesktopClientMsg::Key {
                code: "Unknown".into(),
                key: Some("ř".into()),
                down: true,
            },
            &mut state,
        );
        assert!(matches!(ops[0], Operation::UnicodeKeyPressed('ř')));
    }

    #[test]
    fn wheel_direction_follows_browser_sign_convention() {
        let mut state = 0u8;
        let ops = to_operations(
            DesktopClientMsg::Wheel {
                dx: 0.0,
                dy: -120.0,
            },
            &mut state,
        );
        let Operation::WheelRotations(w) = &ops[0] else {
            panic!("expected wheel op");
        };
        assert!(w.is_vertical);
        assert_eq!(w.rotation_units, WHEEL_NOTCH); // scroll up = positive

        let ops = to_operations(DesktopClientMsg::Wheel { dx: 0.0, dy: 120.0 }, &mut state);
        let Operation::WheelRotations(w) = &ops[0] else {
            panic!("expected wheel op");
        };
        assert_eq!(w.rotation_units, -WHEEL_NOTCH);
    }

    #[test]
    fn cad_presses_then_releases_in_reverse() {
        let mut state = 0u8;
        let ops = to_operations(DesktopClientMsg::Cad, &mut state);
        assert_eq!(ops.len(), 6);
        assert!(matches!(ops[0], Operation::KeyPressed(_)));
        assert!(matches!(ops[5], Operation::KeyReleased(_)));
    }
}
