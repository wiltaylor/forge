//! Remote desktop viewer (features `vnc`/`rdp`): forge-core's VNC/RDP session
//! engines pumped over the in-process widget bridge, blitting raw RGBA rect
//! frames into an egui texture — the egui sibling of `@forge/desktop` (web).
//!
//! State and view are split Forge-style: [`DesktopState`] owns the session
//! (channels, framebuffer texture, status) and is created with
//! [`DesktopState::vnc`]/[`DesktopState::rdp`]; [`DesktopView`] is the
//! builder view:
//!
//! ```ignore
//! // once:
//! let mut desk = DesktopState::vnc(ui.ctx(), VncTarget::new("127.0.0.1"));
//! // per frame:
//! DesktopView::new().scale(ScaleMode::Fit).show(ui, &mut desk);
//! ```
//!
//! Click the well to capture the keyboard (Tab/arrows/Esc are locked to the
//! remote desktop); **Ctrl+Shift+Q** releases the capture. Ctrl+Alt+Del is
//! never sent as a chord — call [`DesktopState::send_cad`] from a toolbar
//! button instead.
//!
//! The session is negotiated raw-only (`encodings: []`, lossless): both
//! engine and widget live in this process, so wire compression would only
//! burn CPU — no decompression dependencies UI-side.

mod keys;

use std::sync::Arc;
use std::time::Duration;

use egui::{
    Align2, Color32, ColorImage, CornerRadius, EventFilter, Key, Rect, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, WidgetInfo, WidgetType,
};
use forge_core::widgets::proto::{
    DesktopClientMsg, DesktopServerMsg, QualityMode, RECT_ENCODING_RAW, RECT_HEADER_LEN,
    RECT_VERSION,
};
use forge_core::widgets::{DesktopConfig, WidgetMsg};
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{scrim, Theme};
use crate::widgets::primitives::Button;
use crate::widgets::stream::{self, SessionChannels};

/// Largest framebuffer side we will allocate a texture for. Anything bigger
/// is treated as a protocol error instead of an allocation panic.
const MAX_FB_SIDE: u16 = 8192;
/// Browser wheel-delta factors per [`egui::MouseWheelUnit`]: a Line is ~40
/// CSS px, a Page ~400 (the classic browser constants the protocol's sign
/// convention was written against).
const WHEEL_LINE: f32 = 40.0;
const WHEEL_PAGE: f32 = 400.0;

/// Where the session is in its lifecycle. `Error`/`Closed` are terminal but
/// keep the last frame visible under an overlay; [`DesktopState::reconnect`]
/// re-opens with the retained target.
#[derive(Clone, Debug, PartialEq)]
pub enum DesktopStatus {
    /// Session opened, waiting for the engine's `ready` frame.
    Connecting,
    /// Live: rect frames flow in, input flows out.
    Ready,
    /// The engine reported an error (connect/auth/protocol failure).
    Error(String),
    /// The remote ended the session, or [`DesktopState::disconnect`] ran.
    Closed,
}

/// How the framebuffer maps into the well.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleMode {
    /// Aspect-preserving letterbox (default).
    #[default]
    Fit,
    /// Native size, centered (crops when larger than the well). Painted with
    /// nearest-neighbour filtering for pixel-exact output.
    OneToOne,
    /// Fill the well, ignoring aspect.
    Stretch,
}

/// VNC connection parameters for [`DesktopState::vnc`].
#[cfg(feature = "vnc")]
pub struct VncTarget {
    pub host: String,
    /// Default 5900 (see [`VncTarget::new`]).
    pub port: u16,
    /// VncAuth password, when the server demands one.
    pub password: Option<String>,
}

#[cfg(feature = "vnc")]
impl VncTarget {
    pub fn new(host: impl Into<String>) -> VncTarget {
        VncTarget {
            host: host.into(),
            port: 5900,
            password: None,
        }
    }
}

/// RDP connection parameters for [`DesktopState::rdp`]. Use `"DOMAIN\\user"`
/// as the username to select an explicit domain.
#[cfg(feature = "rdp")]
pub struct RdpTarget {
    pub host: String,
    /// Default 3389 (see [`RdpTarget::new`]).
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[cfg(feature = "rdp")]
impl RdpTarget {
    pub fn new(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> RdpTarget {
        RdpTarget {
            host: host.into(),
            port: 3389,
            username: username.into(),
            password: password.into(),
        }
    }
}

/// The retained connect target — enough to serialize the `connect` frame and
/// to [`DesktopState::reconnect`] a finished session. Retains the password
/// by design (reconnect UX); never logged (no `Debug`).
enum Target {
    #[cfg(feature = "vnc")]
    Vnc(VncTarget),
    #[cfg(feature = "rdp")]
    Rdp(RdpTarget),
}

impl Target {
    /// The opening control frame. Raw-only/lossless: engine and widget share
    /// a process, so the negotiated wire compression would be pure overhead.
    fn connect_msg(&self) -> DesktopClientMsg {
        let (host, port, username, password) = match self {
            #[cfg(feature = "vnc")]
            Target::Vnc(t) => (t.host.clone(), t.port, None, t.password.clone()),
            #[cfg(feature = "rdp")]
            Target::Rdp(t) => (
                t.host.clone(),
                t.port,
                Some(t.username.clone()),
                Some(t.password.clone()),
            ),
        };
        DesktopClientMsg::Connect {
            host: Some(host),
            port: Some(port),
            username,
            password,
            encodings: Vec::new(),
            quality: QualityMode::Lossless,
            jpeg_quality: None,
        }
    }

    fn spawn(&self, ctx: &egui::Context) -> SessionChannels {
        let config = Arc::new(DesktopConfig::default());
        match self {
            #[cfg(feature = "vnc")]
            Target::Vnc(_) => {
                stream::open_session(ctx, move |s| forge_core::widgets::vnc::session(s, config))
            }
            #[cfg(feature = "rdp")]
            Target::Rdp(_) => {
                stream::open_session(ctx, move |s| forge_core::widgets::rdp::session(s, config))
            }
        }
    }
}

/// Modifier bookkeeping: what the remote currently believes is held.
/// Shift/Ctrl/Alt are synthesized from [`egui::Modifiers`] diffs; Meta is
/// tracked from physical Super key events (`Modifiers` has no super field
/// off macOS).
#[derive(Clone, Copy, Debug, Default)]
struct Held {
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta_l: bool,
    meta_r: bool,
}

/// One desktop session: the engine channels, the framebuffer texture, and
/// the lifecycle status. Owned by the app; render it each frame with
/// [`DesktopView::show`]. Dropping it (or calling
/// [`DesktopState::disconnect`]) closes the engine's inbox, which tears down
/// the VNC/RDP connection.
pub struct DesktopState {
    chan: Option<SessionChannels>,
    texture: Option<TextureHandle>,
    /// Framebuffer size in pixels, from the last `ready`/`resize` frame.
    fb_size: Option<(u16, u16)>,
    status: DesktopStatus,
    target: Target,
    /// Texture filtering, driven by the view's [`ScaleMode`] each frame.
    options: TextureOptions,
    held: Held,
    /// Last `buttons` mask sent (`PointerEvent.buttons`: L=1, R=2, M=4).
    last_buttons: u8,
    /// Last framebuffer pointer position sent.
    last_pointer: Option<(u16, u16)>,
    /// Whether the previous frame held the keyboard capture.
    was_focused: bool,
}

impl DesktopState {
    /// Open a VNC session and send the `connect` frame immediately. The
    /// target (including its password) is retained for
    /// [`DesktopState::reconnect`]; it is never logged.
    #[cfg(feature = "vnc")]
    pub fn vnc(ctx: &egui::Context, target: VncTarget) -> DesktopState {
        DesktopState::open(ctx, Target::Vnc(target))
    }

    /// Open an RDP session and send the `connect` frame immediately. The
    /// target (including its credentials) is retained for
    /// [`DesktopState::reconnect`]; it is never logged.
    #[cfg(feature = "rdp")]
    pub fn rdp(ctx: &egui::Context, target: RdpTarget) -> DesktopState {
        DesktopState::open(ctx, Target::Rdp(target))
    }

    fn open(ctx: &egui::Context, target: Target) -> DesktopState {
        let mut state = DesktopState {
            chan: Some(target.spawn(ctx)),
            texture: None,
            fb_size: None,
            status: DesktopStatus::Connecting,
            target,
            options: TextureOptions::LINEAR,
            held: Held::default(),
            last_buttons: 0,
            last_pointer: None,
            was_focused: false,
        };
        let connect = state.target.connect_msg();
        state.send_ctrl(&connect);
        state
    }

    pub fn status(&self) -> &DesktopStatus {
        &self.status
    }

    /// Framebuffer size in pixels, once the session is ready.
    pub fn fb_size(&self) -> Option<(u16, u16)> {
        self.fb_size
    }

    /// Ctrl+Alt+Del: the engine synthesizes the three-key press/release.
    /// Never bound to a local chord — call this from a toolbar button.
    pub fn send_cad(&mut self) {
        self.send_ctrl(&DesktopClientMsg::Cad);
    }

    /// Release any held modifiers, then drop the session channels, ending
    /// the session. Status becomes [`DesktopStatus::Closed`] unless the
    /// session already ended with an error.
    pub fn disconnect(&mut self) {
        self.release_held();
        self.chan = None;
        if matches!(
            self.status,
            DesktopStatus::Connecting | DesktopStatus::Ready
        ) {
            self.status = DesktopStatus::Closed;
        }
    }

    /// Re-open a finished session from the retained target: fresh channels,
    /// fresh `connect` frame. The last frame stays visible under the
    /// connecting overlay until the new `ready` arrives.
    pub fn reconnect(&mut self, ctx: &egui::Context) {
        self.chan = Some(self.target.spawn(ctx));
        self.status = DesktopStatus::Connecting;
        self.held = Held::default();
        self.last_buttons = 0;
        self.last_pointer = None;
        let connect = self.target.connect_msg();
        self.send_ctrl(&connect);
    }

    /// Drain frames from the engine: binary rect frames into the texture,
    /// control frames into status transitions. Called at the top of every
    /// `show()`.
    fn pump(&mut self, ctx: &egui::Context) {
        loop {
            let Some(chan) = &mut self.chan else { return };
            match chan.rx.try_recv() {
                Ok(WidgetMsg::Binary(frame)) => self.apply_rect(&frame),
                Ok(WidgetMsg::Text(text)) => {
                    match serde_json::from_str::<DesktopServerMsg>(&text) {
                        Ok(DesktopServerMsg::Ready { width, height })
                        | Ok(DesktopServerMsg::Resize { width, height }) => {
                            self.resize_fb(ctx, width, height);
                        }
                        Ok(DesktopServerMsg::Error { message }) => {
                            self.status = DesktopStatus::Error(message);
                        }
                        Ok(DesktopServerMsg::Closed) => self.status = DesktopStatus::Closed,
                        Err(_) => tracing::warn!("ignoring malformed desktop control frame"),
                    }
                }
                Ok(WidgetMsg::Close) | Err(TryRecvError::Disconnected) => {
                    if matches!(
                        self.status,
                        DesktopStatus::Connecting | DesktopStatus::Ready
                    ) {
                        self.status = DesktopStatus::Closed;
                    }
                    self.chan = None;
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    /// `ready`/`resize`: allocate a fresh framebuffer texture, filled with
    /// the well color until rects arrive.
    fn resize_fb(&mut self, ctx: &egui::Context, width: u16, height: u16) {
        if width == 0 || height == 0 || width > MAX_FB_SIDE || height > MAX_FB_SIDE {
            self.status =
                DesktopStatus::Error(format!("unsupported framebuffer size {width}x{height}"));
            self.texture = None;
            self.fb_size = None;
            return;
        }
        let t = Theme::of(ctx);
        let image = ColorImage::filled([width as usize, height as usize], t.bg[1]);
        self.texture = Some(ctx.load_texture("forge-desktop", image, self.options));
        self.fb_size = Some((width, height));
        self.status = DesktopStatus::Ready;
        self.last_pointer = None;
    }

    /// Apply one binary rect frame (docs/widgets-protocol.md layout) in
    /// arrival order. Unknown versions and un-negotiated encodings are
    /// ignored, as are malformed or out-of-bounds rects.
    fn apply_rect(&mut self, frame: &[u8]) {
        let Some((x, y, w, h, payload)) = parse_raw_rect(frame) else {
            return;
        };
        let Some((fb_w, fb_h)) = self.fb_size else {
            return;
        };
        if u32::from(x) + u32::from(w) > u32::from(fb_w)
            || u32::from(y) + u32::from(h) > u32::from(fb_h)
        {
            tracing::debug!("dropping out-of-bounds desktop rect");
            return;
        }
        let Some(texture) = &mut self.texture else {
            return;
        };
        let image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], payload);
        texture.set_partial([x as usize, y as usize], image, self.options);
    }

    /// Diff the frame's modifier state against the bookkeeping and emit the
    /// transitions — called BEFORE forwarding that frame's key events, so
    /// e.g. Shift is down remotely before the shifted key arrives.
    fn sync_modifiers(&mut self, mods: egui::Modifiers) {
        if mods.shift != self.held.shift {
            self.held.shift = mods.shift;
            self.send_key(keys::MOD_SHIFT, None, mods.shift);
        }
        if mods.ctrl != self.held.ctrl {
            self.held.ctrl = mods.ctrl;
            self.send_key(keys::MOD_CTRL, None, mods.ctrl);
        }
        if mods.alt != self.held.alt {
            self.held.alt = mods.alt;
            self.send_key(keys::MOD_ALT, None, mods.alt);
        }
    }

    /// Release everything the remote believes is held — on focus loss,
    /// capture release, and disconnect, so no modifier is left stuck.
    fn release_held(&mut self) {
        let held = std::mem::take(&mut self.held);
        for (down, code) in [
            (held.shift, keys::MOD_SHIFT),
            (held.ctrl, keys::MOD_CTRL),
            (held.alt, keys::MOD_ALT),
            (held.meta_l, "MetaLeft"),
            (held.meta_r, "MetaRight"),
        ] {
            if down {
                self.send_key(code, None, false);
            }
        }
    }

    fn send_key(&mut self, code: &str, key: Option<char>, down: bool) -> bool {
        self.send_ctrl(&DesktopClientMsg::Key {
            code: code.to_owned(),
            key: key.map(String::from),
            down,
        })
    }

    fn send_ctrl(&mut self, msg: &DesktopClientMsg) -> bool {
        let text = serde_json::to_string(msg).expect("DesktopClientMsg serializes");
        self.send_msg(WidgetMsg::Text(text))
    }

    /// UI-thread send: `try_send` only. A full channel means the engine is
    /// wedged behind backpressure — drop the frame and warn (never log the
    /// frame itself: control frames may carry credentials).
    fn send_msg(&mut self, msg: WidgetMsg) -> bool {
        let Some(chan) = &self.chan else { return false };
        match chan.tx.try_send(msg) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                tracing::warn!("desktop session channel full; dropping input frame");
                false
            }
            Err(TrySendError::Closed(_)) => false,
        }
    }
}

/// Where the framebuffer paints inside the well, per [`ScaleMode`].
fn image_rect(scale: ScaleMode, well: Rect, fb: (u16, u16)) -> Rect {
    let fb_size = Vec2::new(f32::from(fb.0), f32::from(fb.1));
    match scale {
        ScaleMode::Stretch => well,
        ScaleMode::OneToOne => Rect::from_center_size(well.center(), fb_size),
        ScaleMode::Fit => {
            let s = (well.width() / fb_size.x)
                .min(well.height() / fb_size.y)
                .max(0.0);
            Rect::from_center_size(well.center(), fb_size * s)
        }
    }
}

/// Parse one raw rect frame via the proto constants. `None` = ignore the
/// frame (unknown version, un-negotiated encoding, or malformed payload).
fn parse_raw_rect(frame: &[u8]) -> Option<(u16, u16, u16, u16, &[u8])> {
    if frame.len() < RECT_HEADER_LEN || frame[0] != RECT_VERSION {
        return None;
    }
    if frame[1] != RECT_ENCODING_RAW {
        // We advertised `encodings: []` (raw only); anything else is a bug.
        tracing::debug!(encoding = frame[1], "ignoring un-negotiated rect encoding");
        return None;
    }
    let le = |i: usize| u16::from_le_bytes([frame[i], frame[i + 1]]);
    let (x, y, w, h) = (le(2), le(4), le(6), le(8));
    let payload = &frame[RECT_HEADER_LEN..];
    if w == 0 || h == 0 || payload.len() != w as usize * h as usize * 4 {
        return None;
    }
    Some((x, y, w, h, payload))
}

/// The desktop view: a bordered well the framebuffer texture paints into.
/// Builder + `show(ui, &mut DesktopState)`, like every Forge widget.
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopView {
    scale: ScaleMode,
}

impl DesktopView {
    pub fn new() -> DesktopView {
        DesktopView::default()
    }

    /// How the framebuffer maps into the well (default [`ScaleMode::Fit`]).
    pub fn scale(mut self, scale: ScaleMode) -> Self {
        self.scale = scale;
        self
    }

    pub fn show(self, ui: &mut Ui, state: &mut DesktopState) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        state.options = match self.scale {
            ScaleMode::OneToOne => TextureOptions::NEAREST,
            _ => TextureOptions::LINEAR,
        };
        state.pump(ui.ctx());

        // The well tracks the framebuffer aspect (16:10 placeholder until
        // `ready`), height-capped so oversized desktops letterbox instead of
        // scrolling the page.
        let fb = state.fb_size.unwrap_or((1280, 800));
        let width = ui.available_width().max(240.0);
        let height = (width * f32::from(fb.1) / f32::from(fb.0)).clamp(160.0, 800.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "remote desktop"));

        if response.clicked() {
            response.request_focus();
        }
        let focused = response.has_focus();

        let well = rect.shrink(1.0);
        let img_rect = state.fb_size.map(|fb| image_rect(self.scale, well, fb));

        let mut outcome = Outcome::Ignored;
        if focused {
            if handle_keyboard(ui, state, &response) {
                outcome = Outcome::Consumed;
            }
        } else if state.was_focused {
            // Capture lost (click-away, Tab-out): nothing stays stuck down.
            state.release_held();
        }
        // Re-read after input: Ctrl+Shift+Q surrenders focus this frame.
        let focused = response.has_focus();
        state.was_focused = focused;

        if let Some(img_rect) = img_rect {
            if handle_pointer(ui, state, &response, img_rect) {
                outcome = outcome.merge(Outcome::Consumed);
            }
        }

        if ui.is_rect_visible(rect) {
            paint(ui, &t, state, rect, img_rect, focused);
        }

        // Closed scrim gets a Reconnect button (errors keep the toolbar in
        // charge — the message banner explains why).
        if state.status == DesktopStatus::Closed {
            let btn_rect = Rect::from_center_size(
                rect.center() + Vec2::new(0.0, 30.0),
                Vec2::new(120.0, t.control.sm),
            );
            let clicked = ui
                .scope_builder(UiBuilder::new().max_rect(btn_rect), |ui| {
                    Button::new("Reconnect").small(true).show(ui).clicked()
                })
                .inner;
            if clicked {
                state.reconnect(ui.ctx());
                outcome = outcome.merge(Outcome::Submitted);
            }
        }

        ForgeResponse::new(response, outcome)
    }
}

/// Forward this frame's captured keyboard input; returns whether anything
/// was sent. Modifier transitions go out before the frame's key events.
fn handle_keyboard(ui: &mut Ui, state: &mut DesktopState, response: &egui::Response) -> bool {
    // Keep Tab/arrows/Esc on the desktop instead of moving egui focus.
    ui.memory_mut(|m| {
        m.set_focus_lock_filter(
            response.id,
            EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });

    state.sync_modifiers(ui.input(|i| i.modifiers));

    let mut sent = false;
    let mut release = false;
    for event in ui.input(|i| i.events.clone()) {
        let egui::Event::Key {
            key,
            physical_key,
            pressed,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        // The capture-escape chord — never forwarded.
        if key == Key::Q && pressed && modifiers.ctrl && modifiers.shift {
            release = true;
            continue;
        }
        // Prefer the physical key: the protocol's `code` field is
        // layout-independent by contract.
        let k = physical_key.unwrap_or(key);
        match k {
            // Covered by the Modifiers diff above.
            Key::ShiftLeft
            | Key::ShiftRight
            | Key::ControlLeft
            | Key::ControlRight
            | Key::AltLeft
            | Key::AltRight => continue,
            // Meta has no Modifiers field off macOS: track the physical
            // events so release-on-focus-loss covers it too.
            Key::SuperLeft => state.held.meta_l = pressed,
            Key::SuperRight => state.held.meta_r = pressed,
            _ => {}
        }
        let Some(code) = keys::code_str(k) else {
            continue;
        };
        // Key-repeats arrive as extra `pressed` events and are forwarded
        // as extra downs — the protocol tracks both edges.
        sent |= state.send_key(code, keys::us_char(k, modifiers.shift), pressed);
    }

    if release {
        state.release_held();
        response.surrender_focus();
    }
    sent
}

/// Pointer + wheel: inverse-map into framebuffer coordinates and send only
/// on change. Returns whether anything was sent.
fn handle_pointer(
    ui: &Ui,
    state: &mut DesktopState,
    response: &egui::Response,
    img_rect: Rect,
) -> bool {
    let Some((fb_w, fb_h)) = state.fb_size else {
        return false;
    };
    let hovered = response.hovered();
    // Track through drags that leave the well, and until the release after
    // a press that started here — but never buttons pressed elsewhere.
    if !hovered && !response.dragged() && state.last_buttons == 0 {
        return false;
    }
    let mut sent = false;

    let mask = if response.is_pointer_button_down_on() || state.last_buttons != 0 {
        ui.input(|i| {
            u8::from(i.pointer.primary_down())
                | (u8::from(i.pointer.secondary_down()) << 1)
                | (u8::from(i.pointer.middle_down()) << 2)
        })
    } else {
        0
    };

    let pos = response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos());
    if let Some(pos) = pos {
        if img_rect.width() > 0.0 && img_rect.height() > 0.0 {
            let fx = (pos.x - img_rect.min.x) / img_rect.width() * f32::from(fb_w);
            let fy = (pos.y - img_rect.min.y) / img_rect.height() * f32::from(fb_h);
            let fb_pos = (
                fx.clamp(0.0, f32::from(fb_w - 1)) as u16,
                fy.clamp(0.0, f32::from(fb_h - 1)) as u16,
            );
            let changed = Some(fb_pos) != state.last_pointer || mask != state.last_buttons;
            if changed
                && state.send_ctrl(&DesktopClientMsg::Mouse {
                    x: fb_pos.0,
                    y: fb_pos.1,
                    buttons: mask,
                })
            {
                state.last_pointer = Some(fb_pos);
                state.last_buttons = mask;
                sent = true;
            }
        }
    }

    // Wheel: egui's delta is how the CONTENT moves (positive = down/right
    // revealed above/left); browsers report the opposite sign.
    if hovered {
        for event in ui.input(|i| i.events.clone()) {
            let egui::Event::MouseWheel { unit, delta, .. } = event else {
                continue;
            };
            if delta == Vec2::ZERO {
                continue;
            }
            let f = match unit {
                egui::MouseWheelUnit::Point => 1.0,
                egui::MouseWheelUnit::Line => WHEEL_LINE,
                egui::MouseWheelUnit::Page => WHEEL_PAGE,
            };
            sent |= state.send_ctrl(&DesktopClientMsg::Wheel {
                dx: f64::from(-delta.x * f),
                dy: f64::from(-delta.y * f),
            });
        }
    }
    sent
}

/// Paint the well, framebuffer, capture badge, and status overlays.
fn paint(
    ui: &Ui,
    t: &Theme,
    state: &DesktopState,
    rect: Rect,
    img_rect: Option<Rect>,
    focused: bool,
) {
    let radius = CornerRadius::same(t.radius.md as u8);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, t.bg[1]);
    let border = if focused {
        t.accent.base
    } else {
        t.border.default
    };
    painter.rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);

    if let (Some(texture), Some(img_rect)) = (&state.texture, img_rect) {
        let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter
            .with_clip_rect(rect.shrink(1.0))
            .image(texture.id(), img_rect, uv, Color32::WHITE);
    }

    // Capture badge: how to get the keyboard back.
    if focused {
        let font = t.mono(t.type_scale.xs);
        let galley =
            painter.layout_no_wrap("▣ captured · Ctrl+Shift+Q releases".into(), font, t.fg[2]);
        let pad = Vec2::new(6.0, 3.0);
        let size = galley.size() + pad * 2.0;
        let chip = Rect::from_min_size(
            egui::pos2(rect.max.x - size.x - 6.0, rect.min.y + 6.0),
            size,
        );
        painter.rect_filled(chip, CornerRadius::same(t.radius.sm as u8), t.bg[3]);
        painter.galley(chip.min + pad, galley, t.fg[2]);
    }

    // Status overlays inside the well.
    match &state.status {
        DesktopStatus::Ready => {}
        DesktopStatus::Connecting => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "connecting…",
                t.mono(t.type_scale.sm),
                t.fg[2],
            );
            // Keep frames coming while we wait for the engine.
            ui.ctx().request_repaint_after(Duration::from_millis(120));
        }
        DesktopStatus::Closed => {
            painter.rect_filled(rect, radius, scrim(t));
            painter.text(
                rect.center() - Vec2::new(0.0, 10.0),
                Align2::CENTER_CENTER,
                "session closed",
                t.mono(t.type_scale.sm),
                t.fg[1],
            );
        }
        DesktopStatus::Error(message) => {
            let banner = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 26.0));
            let r = t.radius.md as u8;
            let top_radius = CornerRadius {
                nw: r,
                ne: r,
                sw: 0,
                se: 0,
            };
            painter.rect_filled(banner, top_radius, t.danger.bg);
            painter.with_clip_rect(banner).text(
                egui::pos2(banner.min.x + 8.0, banner.center().y),
                Align2::LEFT_CENTER,
                message,
                t.mono(t.type_scale.xs),
                t.danger.fg,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core::widgets::proto::encode_rect;
    use std::time::Instant;

    #[test]
    fn rect_frames_parse_via_the_proto_constants() {
        let rgba = [1u8, 2, 3, 4, 5, 6, 7, 8]; // 2x1 px
        let frame = encode_rect(7, 9, 2, 1, &rgba);
        let (x, y, w, h, payload) = parse_raw_rect(&frame).expect("valid rect frame");
        assert_eq!((x, y, w, h), (7, 9, 2, 1));
        assert_eq!(payload, &[1, 2, 3, 0xFF, 5, 6, 7, 0xFF]);

        // Unknown version → ignored, not an error.
        let mut wrong_version = frame.clone();
        wrong_version[0] = RECT_VERSION + 1;
        assert!(parse_raw_rect(&wrong_version).is_none());
        // Un-negotiated encoding → ignored.
        let mut wrong_encoding = frame.clone();
        wrong_encoding[1] = 1; // deflate: never advertised
        assert!(parse_raw_rect(&wrong_encoding).is_none());
        // Truncated payload → ignored.
        assert!(parse_raw_rect(&frame[..frame.len() - 1]).is_none());
        assert!(parse_raw_rect(&frame[..RECT_HEADER_LEN - 1]).is_none());
    }

    #[test]
    fn letterbox_math() {
        let well = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(800.0, 300.0));

        // Fit: height-bound here (800x300 well, 4:3 fb) → 400x300 centered.
        let r = image_rect(ScaleMode::Fit, well, (1024, 768));
        assert_eq!(r.size(), Vec2::new(400.0, 300.0));
        assert_eq!(r.center(), well.center());
        // Width-bound: wide fb in a squarer well.
        let r = image_rect(ScaleMode::Fit, well, (1600, 100));
        assert_eq!(r.size(), Vec2::new(800.0, 50.0));

        // OneToOne: native size, centered (may exceed the well; clipped at
        // paint time).
        let r = image_rect(ScaleMode::OneToOne, well, (1024, 768));
        assert_eq!(r.size(), Vec2::new(1024.0, 768.0));
        assert_eq!(r.center(), well.center());

        // Stretch: the well itself.
        assert_eq!(image_rect(ScaleMode::Stretch, well, (1024, 768)), well);
    }

    /// End-to-end over the real engine + lazy runtime, no server needed:
    /// the constructor's connect frame reaches the VNC engine, whose TCP
    /// connect to a closed loopback port fails → an `error` control frame
    /// comes back and the status reflects it.
    #[cfg(feature = "vnc")]
    #[test]
    fn vnc_engine_reports_connect_failure_end_to_end() {
        let deadline = Instant::now() + Duration::from_secs(15);
        let ctx = egui::Context::default();
        // Port 1 on loopback: nothing listens there → immediate refusal.
        let mut target = VncTarget::new("127.0.0.1");
        target.port = 1;
        let mut state = DesktopState::vnc(&ctx, target);

        loop {
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 640.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run_ui(raw, |ui| {
                let _ = DesktopView::new().show(ui, &mut state);
            });
            match state.status() {
                DesktopStatus::Error(message) => {
                    assert!(message.contains("vnc connect"), "unexpected: {message}");
                    break;
                }
                DesktopStatus::Ready => panic!("connected to a closed port?"),
                _ => {}
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for the error frame; status={:?}",
                state.status()
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        // Terminal state: the channels are gone after the engine's Close.
        state.disconnect();
        assert!(matches!(state.status(), DesktopStatus::Error(_)));
    }
}
