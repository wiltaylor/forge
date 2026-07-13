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
        /// Rect encodings the client can decode, as header-byte values
        /// ([`RECT_ENCODING_RAW`]...). Absent/empty = raw only, so
        /// pre-negotiation clients keep working unchanged.
        #[serde(default)]
        encodings: Vec<u8>,
        /// The server may emit lossy (JPEG) rects only in `lossy` mode.
        #[serde(default)]
        quality: QualityMode,
        /// JPEG quality 1..=100 (clamped, default 75). Lossy mode only.
        #[serde(default)]
        jpeg_quality: Option<u8>,
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

/// Client quality preference from the `connect` frame: lossless keeps every
/// rect pixel-exact (raw or deflate); lossy additionally allows JPEG rects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityMode {
    #[default]
    Lossless,
    Lossy,
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
/// Rect encoding: raw RGBA.
pub const RECT_ENCODING_RAW: u8 = 0;
/// Rect encoding: raw deflate (RFC 1951, no zlib wrapper) of the alpha-forced
/// RGBA payload. The decompressed length is exactly `w*h*4`.
pub const RECT_ENCODING_DEFLATE: u8 = 1;
/// Rect encoding: baseline JPEG of the rect (lossy; alpha discarded).
pub const RECT_ENCODING_JPEG: u8 = 2;
/// Rect frame header size: version, encoding, then x/y/w/h as u16 LE.
pub const RECT_HEADER_LEN: usize = 10;

/// Write the 10-byte rect frame header into a Vec sized for the payload.
fn header(encoding: u8, x: u16, y: u16, w: u16, h: u16, payload_cap: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(RECT_HEADER_LEN + payload_cap);
    buf.push(RECT_VERSION);
    buf.push(encoding);
    buf.extend_from_slice(&x.to_le_bytes());
    buf.extend_from_slice(&y.to_le_bytes());
    buf.extend_from_slice(&w.to_le_bytes());
    buf.extend_from_slice(&h.to_le_bytes());
    buf
}

/// Encode one framebuffer update rect: 10-byte LE header + `w*h*4` RGBA bytes.
/// The alpha byte is forced to 0xFF: both protocol decoders emit padding (VNC
/// RGBX, RDP bitmaps) there, and a 0 alpha blits as fully transparent.
pub fn encode_rect(x: u16, y: u16, w: u16, h: u16, rgba: &[u8]) -> Vec<u8> {
    debug_assert_eq!(rgba.len(), w as usize * h as usize * 4);
    let mut buf = header(RECT_ENCODING_RAW, x, y, w, h, rgba.len());
    buf.extend_from_slice(rgba);
    for px in buf[RECT_HEADER_LEN..].chunks_exact_mut(4) {
        px[3] = 0xFF;
    }
    buf
}

/// Rects at or below this raw payload size (16x16 px) skip compression: the
/// win can't outweigh the per-rect CPU and format overhead.
#[cfg(any(feature = "vnc", feature = "rdp"))]
const RAW_CUTOFF: usize = 1024;
/// Minimum rect side for JPEG; below this the format overhead dominates.
#[cfg(any(feature = "vnc", feature = "rdp"))]
const JPEG_MIN_SIDE: u16 = 16;
#[cfg(any(feature = "vnc", feature = "rdp"))]
const JPEG_DEFAULT_QUALITY: u8 = 75;

/// Per-session rect encoder honouring the encodings negotiated in the
/// `connect` frame. Emits the smallest winning representation per rect and
/// falls back to raw whenever compression doesn't pay — so the output is
/// never larger than the [`encode_rect`] frame for the same input.
#[cfg(any(feature = "vnc", feature = "rdp"))]
pub struct RectEncoder {
    deflate: bool,
    jpeg: bool,
    jpeg_quality: u8,
    /// Reusable alpha-forced RGBA staging buffer.
    scratch: Vec<u8>,
}

#[cfg(any(feature = "vnc", feature = "rdp"))]
impl RectEncoder {
    /// Pre-negotiation sessions: byte-identical to [`encode_rect`].
    pub fn raw_only() -> Self {
        Self {
            deflate: false,
            jpeg: false,
            jpeg_quality: JPEG_DEFAULT_QUALITY,
            scratch: Vec::new(),
        }
    }

    /// Build from the `connect` frame's negotiation fields. Unknown encoding
    /// values are ignored; JPEG additionally requires `quality: "lossy"`.
    pub fn from_connect(encodings: &[u8], quality: QualityMode, jpeg_quality: Option<u8>) -> Self {
        Self {
            deflate: encodings.contains(&RECT_ENCODING_DEFLATE),
            jpeg: quality == QualityMode::Lossy && encodings.contains(&RECT_ENCODING_JPEG),
            jpeg_quality: jpeg_quality.unwrap_or(JPEG_DEFAULT_QUALITY).clamp(1, 100),
            scratch: Vec::new(),
        }
    }

    /// Encode one rect frame (same header/alpha semantics as [`encode_rect`]).
    pub fn encode(&mut self, x: u16, y: u16, w: u16, h: u16, rgba: &[u8]) -> Vec<u8> {
        debug_assert_eq!(rgba.len(), w as usize * h as usize * 4);
        self.scratch.clear();
        self.scratch.extend_from_slice(rgba);
        for px in self.scratch.chunks_exact_mut(4) {
            px[3] = 0xFF;
        }
        let raw_len = self.scratch.len();

        // Candidate representations; keep the smallest that beats raw. Flat
        // regions deflate far below any JPEG, so try both when negotiated.
        let mut best: Option<(u8, Vec<u8>)> = None;
        if raw_len > RAW_CUTOFF {
            if self.jpeg && w >= JPEG_MIN_SIDE && h >= JPEG_MIN_SIDE {
                let mut out = Vec::new();
                let enc = jpeg_encoder::Encoder::new(&mut out, self.jpeg_quality);
                if enc
                    .encode(&self.scratch, w, h, jpeg_encoder::ColorType::Rgba)
                    .is_ok()
                    && out.len() < raw_len
                {
                    best = Some((RECT_ENCODING_JPEG, out));
                }
            }
            if self.deflate {
                use std::io::Write as _;
                let mut enc =
                    flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
                if let Ok(deflated) = enc.write_all(&self.scratch).and_then(|()| enc.finish()) {
                    if deflated.len() < raw_len
                        && best.as_ref().is_none_or(|(_, b)| deflated.len() < b.len())
                    {
                        best = Some((RECT_ENCODING_DEFLATE, deflated));
                    }
                }
            }
        }

        let (encoding, payload) = match &best {
            Some((encoding, payload)) => (*encoding, payload.as_slice()),
            None => (RECT_ENCODING_RAW, self.scratch.as_slice()),
        };
        let mut buf = header(encoding, x, y, w, h, payload.len());
        buf.extend_from_slice(payload);
        buf
    }
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
                encodings: Vec::new(),
                quality: QualityMode::Lossless,
                jpeg_quality: None,
            }
        );

        // A negotiating client advertises encodings and a quality mode.
        let connect: DesktopClientMsg = from_str(
            r#"{"type":"connect","host":"vm","encodings":[0,1,2],
                "quality":"lossy","jpeg_quality":60}"#,
        )
        .unwrap();
        assert_eq!(
            connect,
            DesktopClientMsg::Connect {
                host: Some("vm".into()),
                port: None,
                username: None,
                password: None,
                encodings: vec![0, 1, 2],
                quality: QualityMode::Lossy,
                jpeg_quality: Some(60),
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
    fn encode_rect_is_byte_exact_and_forces_opaque_alpha() {
        let rgba = [1u8, 2, 3, 4, 5, 6, 7, 8]; // 2x1 pixels, padding alpha
        let frame = encode_rect(0x0102, 0x0304, 2, 1, &rgba);
        assert_eq!(frame.len(), RECT_HEADER_LEN + rgba.len());
        assert_eq!(
            &frame[..RECT_HEADER_LEN],
            &[1, 0, 0x02, 0x01, 0x04, 0x03, 2, 0, 1, 0]
        );
        assert_eq!(&frame[RECT_HEADER_LEN..], &[1, 2, 3, 0xFF, 5, 6, 7, 0xFF]);
    }

    #[cfg(any(feature = "vnc", feature = "rdp"))]
    mod rect_encoder {
        use super::*;

        /// A rect big enough to clear RAW_CUTOFF (64x64 = 16 KiB raw).
        const W: u16 = 64;
        const H: u16 = 64;

        fn solid(rgb: [u8; 3]) -> Vec<u8> {
            [rgb[0], rgb[1], rgb[2], 0].repeat(W as usize * H as usize)
        }

        /// Deterministic high-entropy pixels that neither codec can shrink.
        fn noise() -> Vec<u8> {
            let mut state = 0x2545_F491_4F6C_DD1Du64;
            (0..W as usize * H as usize * 4)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state >> 32) as u8
                })
                .collect()
        }

        /// Horizontal gradient: JPEG-friendly, deflates poorly at level fast.
        fn gradient() -> Vec<u8> {
            let mut px = Vec::with_capacity(W as usize * H as usize * 4);
            for y in 0..H {
                for x in 0..W {
                    px.extend_from_slice(&[(x * 4) as u8, (y * 4) as u8, 128, 0]);
                }
            }
            px
        }

        fn inflate(payload: &[u8]) -> Vec<u8> {
            use std::io::Read as _;
            let mut out = Vec::new();
            flate2::read::DeflateDecoder::new(payload)
                .read_to_end(&mut out)
                .expect("valid raw-deflate payload");
            out
        }

        #[test]
        fn raw_only_matches_encode_rect() {
            let rgba = noise();
            assert_eq!(
                RectEncoder::raw_only().encode(3, 7, W, H, &rgba),
                encode_rect(3, 7, W, H, &rgba)
            );
        }

        #[test]
        fn deflate_roundtrips_and_wins_on_flat_content() {
            let rgba = solid([10, 20, 30]);
            let mut enc = RectEncoder::from_connect(&[0, 1], QualityMode::Lossless, None);
            let frame = enc.encode(0, 0, W, H, &rgba);
            assert_eq!(frame[1], RECT_ENCODING_DEFLATE);
            assert!(frame.len() < RECT_HEADER_LEN + rgba.len() / 10);
            let expected: Vec<u8> = rgba
                .chunks_exact(4)
                .flat_map(|px| [px[0], px[1], px[2], 0xFF])
                .collect();
            assert_eq!(inflate(&frame[RECT_HEADER_LEN..]), expected);
        }

        #[test]
        fn tiny_rects_stay_raw_and_output_never_exceeds_raw() {
            // 16x16 = exactly RAW_CUTOFF bytes: compression skipped entirely,
            // even though this solid rect would deflate massively.
            let mut enc = RectEncoder::from_connect(&[0, 1, 2], QualityMode::Lossy, None);
            let small = solid([1, 2, 3])[..16 * 16 * 4].to_vec();
            let frame = enc.encode(0, 0, 16, 16, &small);
            assert_eq!(frame[1], RECT_ENCODING_RAW);

            // Worst-case content: whatever representation wins, the frame
            // never exceeds the raw frame for the same rect. (True raw
            // fallback is rare — the forced-0xFF alpha bytes alone give
            // deflate a small win even on noise.)
            let rgba = noise();
            let frame = enc.encode(0, 0, W, H, &rgba);
            assert!(frame.len() <= RECT_HEADER_LEN + rgba.len());
        }

        #[test]
        fn lossy_mode_emits_jpeg_on_gradient_content() {
            let rgba = gradient();
            let mut enc = RectEncoder::from_connect(&[0, 1, 2], QualityMode::Lossy, Some(200));
            let frame = enc.encode(0, 0, W, H, &rgba);
            assert_eq!(frame[1], RECT_ENCODING_JPEG);
            assert!(frame.len() < RECT_HEADER_LEN + rgba.len());

            // The payload is a decodable JPEG of the right size, close to the
            // source (quality clamped from 200 to 100).
            let mut decoder = zune_jpeg::JpegDecoder::new(&frame[RECT_HEADER_LEN..]);
            let pixels = decoder.decode().expect("valid jpeg payload");
            let (jw, jh) = decoder.dimensions().expect("decoded dimensions");
            assert_eq!((jw as u16, jh as u16), (W, H));
            let rgb: Vec<u8> = rgba
                .chunks_exact(4)
                .flat_map(|px| [px[0], px[1], px[2]])
                .collect();
            assert_eq!(pixels.len(), rgb.len());
            let max_err = pixels
                .iter()
                .zip(&rgb)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(max_err <= 16, "jpeg deviates too much: {max_err}");
        }

        #[test]
        fn lossless_mode_never_emits_jpeg() {
            // Encoding 2 advertised, but quality stays lossless.
            let mut enc = RectEncoder::from_connect(&[0, 1, 2], QualityMode::Lossless, None);
            let frame = enc.encode(0, 0, W, H, &gradient());
            assert_ne!(frame[1], RECT_ENCODING_JPEG);
        }

        #[test]
        fn lossy_mode_still_prefers_deflate_on_flat_content() {
            // A solid rect deflates far below any JPEG; the smaller wins.
            let mut enc = RectEncoder::from_connect(&[0, 1, 2], QualityMode::Lossy, None);
            let frame = enc.encode(0, 0, W, H, &solid([200, 100, 50]));
            assert_eq!(frame[1], RECT_ENCODING_DEFLATE);
        }
    }
}
