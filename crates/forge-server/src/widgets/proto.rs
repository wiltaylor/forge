//! Wire protocol for the widget WebSockets (docs/widgets-protocol.md).
//!
//! Each connection speaks JSON text frames for control and binary frames for
//! payload: raw tty bytes on `/api/term`, RGBA rect frames on `/api/desktop/*`.
//! Client messages may carry credentials — never log them.

use serde::{Deserialize, Serialize};

/// Terminal session kind, chosen by the first `start` message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermMode {
    Local,
    Ssh,
}

/// Control frames the terminal widget sends (`/api/term`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TermClientMsg {
    Start {
        mode: TermMode,
        #[serde(default)]
        host: Option<String>,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
        cols: u16,
        rows: u16,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
}

/// Control frames the server sends on `/api/term`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TermServerMsg {
    Ready,
    Exit { code: i32 },
    Error { message: String },
}

/// Control frames the desktop widget sends (`/api/desktop/vnc` + `/api/desktop/rdp`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DesktopClientMsg {
    Connect {
        #[serde(default)]
        host: Option<String>,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
    },
    /// `code` is the layout-independent KeyboardEvent.code; `key` carries the
    /// printable character for the Unicode-keysym path (VNC).
    Key {
        code: String,
        #[serde(default)]
        key: Option<String>,
        down: bool,
    },
    /// Framebuffer coordinates; `buttons` is the PointerEvent.buttons bitmask.
    Mouse {
        x: u16,
        y: u16,
        buttons: u8,
    },
    Wheel {
        dx: f64,
        dy: f64,
    },
    /// Ctrl+Alt+Del: the backend synthesizes the three-key press/release.
    Cad,
}

/// Control frames the server sends on `/api/desktop/*`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DesktopServerMsg {
    Ready { width: u16, height: u16 },
    Resize { width: u16, height: u16 },
    Error { message: String },
    Closed,
}

/// Binary rect frame version byte.
pub const RECT_VERSION: u8 = 1;
/// Rect encoding: raw RGBA. The byte reserves per-rect compression additively.
pub const RECT_ENCODING_RAW: u8 = 0;
/// Rect frame header size: version, encoding, then x/y/w/h as u16 LE.
pub const RECT_HEADER_LEN: usize = 10;

/// Encode one framebuffer update rect: 10-byte LE header + `w*h*4` RGBA bytes.
pub fn encode_rect(x: u16, y: u16, w: u16, h: u16, rgba: &[u8]) -> Vec<u8> {
    debug_assert_eq!(rgba.len(), w as usize * h as usize * 4);
    let mut buf = Vec::with_capacity(RECT_HEADER_LEN + rgba.len());
    buf.push(RECT_VERSION);
    buf.push(RECT_ENCODING_RAW);
    buf.extend_from_slice(&x.to_le_bytes());
    buf.extend_from_slice(&y.to_le_bytes());
    buf.extend_from_slice(&w.to_le_bytes());
    buf.extend_from_slice(&h.to_le_bytes());
    buf.extend_from_slice(rgba);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, json, to_value};

    #[test]
    fn term_start_parses_frontend_frame() {
        // Exactly what packages/term sends for an ssh session.
        let msg: TermClientMsg = from_str(
            r#"{"type":"start","mode":"ssh","host":"h","port":22,
                "username":"u","password":"p","cols":80,"rows":24}"#,
        )
        .unwrap();
        assert_eq!(
            msg,
            TermClientMsg::Start {
                mode: TermMode::Ssh,
                host: Some("h".into()),
                port: Some(22),
                username: Some("u".into()),
                password: Some("p".into()),
                cols: 80,
                rows: 24,
            }
        );
    }

    #[test]
    fn term_start_local_omits_target_fields() {
        // JSON.stringify drops undefined props — host/port/user/pass absent.
        let msg: TermClientMsg =
            from_str(r#"{"type":"start","mode":"local","cols":120,"rows":40}"#).unwrap();
        assert_eq!(
            msg,
            TermClientMsg::Start {
                mode: TermMode::Local,
                host: None,
                port: None,
                username: None,
                password: None,
                cols: 120,
                rows: 40,
            }
        );
    }

    #[test]
    fn term_client_resize_round_trips() {
        let msg = TermClientMsg::Resize {
            cols: 100,
            rows: 30,
        };
        let v = to_value(&msg).unwrap();
        assert_eq!(v, json!({"type":"resize","cols":100,"rows":30}));
        assert_eq!(from_str::<TermClientMsg>(&v.to_string()).unwrap(), msg);
    }

    #[test]
    fn term_server_msgs_serialize_to_widget_shapes() {
        assert_eq!(
            to_value(TermServerMsg::Ready).unwrap(),
            json!({"type":"ready"})
        );
        assert_eq!(
            to_value(TermServerMsg::Exit { code: 130 }).unwrap(),
            json!({"type":"exit","code":130})
        );
        assert_eq!(
            to_value(TermServerMsg::Error {
                message: "boom".into()
            })
            .unwrap(),
            json!({"type":"error","message":"boom"})
        );
    }

    #[test]
    fn desktop_client_msgs_parse_frontend_frames() {
        let connect: DesktopClientMsg =
            from_str(r#"{"type":"connect","host":"vm","port":5900,"username":"u","password":"p"}"#)
                .unwrap();
        assert_eq!(
            connect,
            DesktopClientMsg::Connect {
                host: Some("vm".into()),
                port: Some(5900),
                username: Some("u".into()),
                password: Some("p".into()),
            }
        );

        let key: DesktopClientMsg =
            from_str(r#"{"type":"key","code":"KeyA","key":"a","down":true}"#).unwrap();
        assert_eq!(
            key,
            DesktopClientMsg::Key {
                code: "KeyA".into(),
                key: Some("a".into()),
                down: true
            }
        );

        let mouse: DesktopClientMsg =
            from_str(r#"{"type":"mouse","x":10,"y":20,"buttons":1}"#).unwrap();
        assert_eq!(
            mouse,
            DesktopClientMsg::Mouse {
                x: 10,
                y: 20,
                buttons: 1
            }
        );

        let wheel: DesktopClientMsg = from_str(r#"{"type":"wheel","dx":0,"dy":-102.5}"#).unwrap();
        assert_eq!(
            wheel,
            DesktopClientMsg::Wheel {
                dx: 0.0,
                dy: -102.5
            }
        );

        let cad: DesktopClientMsg = from_str(r#"{"type":"cad"}"#).unwrap();
        assert_eq!(cad, DesktopClientMsg::Cad);
    }

    #[test]
    fn desktop_server_msgs_serialize_to_widget_shapes() {
        assert_eq!(
            to_value(DesktopServerMsg::Ready {
                width: 1280,
                height: 800
            })
            .unwrap(),
            json!({"type":"ready","width":1280,"height":800})
        );
        assert_eq!(
            to_value(DesktopServerMsg::Resize {
                width: 640,
                height: 480
            })
            .unwrap(),
            json!({"type":"resize","width":640,"height":480})
        );
        assert_eq!(
            to_value(DesktopServerMsg::Closed).unwrap(),
            json!({"type":"closed"})
        );
    }

    #[test]
    fn encode_rect_is_byte_exact() {
        let rgba = [1u8, 2, 3, 4, 5, 6, 7, 8]; // 2x1 pixels
        let frame = encode_rect(0x0102, 0x0304, 2, 1, &rgba);
        assert_eq!(frame.len(), RECT_HEADER_LEN + rgba.len());
        assert_eq!(
            &frame[..RECT_HEADER_LEN],
            &[1, 0, 0x02, 0x01, 0x04, 0x03, 2, 0, 1, 0]
        );
        assert_eq!(&frame[RECT_HEADER_LEN..], &rgba);
    }
}
