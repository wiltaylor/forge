//! Embedded terminal (feature `term`): forge-core's PTY/SSH engines pumped
//! over the in-process widget bridge, parsed UI-side by vt100 and painted as
//! a mono glyph grid — the egui sibling of
//! `forge-tui/src/widgets/specialty/terminal.rs` and `packages/term` (web).
//!
//! State and view are split Forge-style: [`TermState`] owns the session
//! (channels, vt100 parser, status) and is created with
//! [`TermState::local`]/[`TermState::ssh`]; [`Terminal`] is the builder view:
//!
//! ```ignore
//! // once:
//! let mut term = TermState::local(ui.ctx());
//! // per frame:
//! Terminal::new().rows(24).show(ui, &mut term);
//! ```
//!
//! Click the well to capture the keyboard (Tab/arrows/Esc are locked to the
//! terminal); **Ctrl+Shift+Q** releases the capture. Mouse clicks, drags, and
//! wheel are forwarded to the session as xterm mouse reports when the running
//! program enables mouse tracking (htop/vim/tmux); a plain shell gets none.
//! Local scrollback/selection are still deferred (forge-tui parity) — outside
//! mouse-tracking apps the wheel maps to arrow keys in alternate-screen apps
//! only.

use std::sync::Arc;
use std::time::Duration;

use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CornerRadius, EventFilter, FontId, Key, Rect, Sense, Stroke, StrokeKind,
    TextFormat, Ui, Vec2, WidgetInfo, WidgetType,
};
use forge_core::widgets::proto::{TermClientMsg, TermMode, TermServerMsg};
use forge_core::widgets::{TermConfig, WidgetMsg};
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{scrim, Theme};
use crate::widgets::stream::{self, SessionChannels};

/// vt100 scrollback lines (forge-tui parity; no scrollback UI in v1).
const SCROLLBACK: usize = 2000;
/// How long the measured grid must hold still before `set_size` + a `resize`
/// frame go out — window drags produce a size per frame otherwise.
const RESIZE_DEBOUNCE: f64 = 0.15;
/// Cursor blink half-period in seconds.
const BLINK: f64 = 0.5;
/// Inner padding between the well border and the glyph grid.
const PAD: f32 = 8.0;
/// The bold mono family registered by `theme::fonts` (its `MONO_BOLD` const
/// is private to `theme`, so the name is mirrored here); guarded by a
/// `definitions()` check so unbound contexts fall back to regular mono.
const MONO_BOLD_FAMILY: &str = "jetbrains-mono-bold";

/// Where the session is in its lifecycle. `Exited`/`Error` are terminal but
/// keep the last screen visible under an overlay; [`TermState::restart`]
/// re-opens with the retained start parameters.
#[derive(Clone, Debug, PartialEq)]
pub enum TermStatus {
    /// Session opened, waiting for the engine's `ready` frame.
    Connecting,
    /// Live: tty bytes flow both ways.
    Ready,
    /// The shell/remote process exited with this code.
    Exited(i32),
    /// The engine reported an error (spawn/connect/auth failure).
    Error(String),
    /// The stream closed without an exit report, or [`TermState::disconnect`]
    /// was called.
    Closed,
}

/// The retained `start` parameters — enough to serialize the first frame and
/// to [`TermState::restart`] a finished session. Carries credentials for SSH;
/// never logged (no `Debug`).
struct StartSpec {
    mode: TermMode,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    config: Arc<TermConfig>,
}

/// SSH connection parameters for [`TermState::ssh`].
#[cfg(feature = "term-ssh")]
pub struct SshOptions {
    pub host: String,
    /// Default 22 (see [`SshOptions::new`]).
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[cfg(feature = "term-ssh")]
impl SshOptions {
    pub fn new(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> SshOptions {
        SshOptions {
            host: host.into(),
            port: 22,
            username: username.into(),
            password: password.into(),
        }
    }
}

/// One terminal session: the engine channels, the vt100 screen, and the
/// lifecycle status. Owned by the app; render it each frame with
/// [`Terminal::show`]. Dropping it (or calling [`TermState::disconnect`])
/// closes the engine's inbox, which kills the PTY/SSH session.
pub struct TermState {
    chan: Option<SessionChannels>,
    parser: vt100::Parser,
    status: TermStatus,
    spec: StartSpec,
    /// Whether the `start` frame went out (the first `show()` that knows the
    /// grid size sends it).
    started: bool,
    /// Current grid as (cols, rows) — the size the engine believes.
    grid: (u16, u16),
    /// Debounced resize target: `((cols, rows), first_seen_time)`.
    pending_resize: Option<((u16, u16), f64)>,
    /// Wheel remainder in points, converted to arrow keys row by row.
    scroll_accum: f32,
    /// Cell of the last reported mouse motion, so button-motion/any-motion
    /// modes only report when the pointer crosses into a new cell.
    last_mouse_cell: Option<(u16, u16)>,
}

impl TermState {
    /// A local shell session with [`TermConfig::default`] (`$SHELL`).
    pub fn local(ctx: &egui::Context) -> TermState {
        TermState::local_with(ctx, TermConfig::default())
    }

    /// A local shell session with an explicit engine config.
    pub fn local_with(ctx: &egui::Context, config: TermConfig) -> TermState {
        TermState::open(
            ctx,
            StartSpec {
                mode: TermMode::Local,
                host: None,
                port: None,
                username: None,
                password: None,
                config: Arc::new(config),
            },
        )
    }

    /// An SSH session (password auth). Credentials are retained only for
    /// [`TermState::restart`] and are never logged.
    #[cfg(feature = "term-ssh")]
    pub fn ssh(ctx: &egui::Context, opts: SshOptions) -> TermState {
        TermState::open(
            ctx,
            StartSpec {
                mode: TermMode::Ssh,
                host: Some(opts.host),
                port: Some(opts.port),
                username: Some(opts.username),
                password: Some(opts.password),
                config: Arc::new(TermConfig::default()),
            },
        )
    }

    fn open(ctx: &egui::Context, spec: StartSpec) -> TermState {
        TermState {
            chan: Some(TermState::spawn(ctx, spec.config.clone())),
            parser: vt100::Parser::new(24, 80, SCROLLBACK),
            status: TermStatus::Connecting,
            spec,
            started: false,
            grid: (80, 24),
            pending_resize: None,
            scroll_accum: 0.0,
            last_mouse_cell: None,
        }
    }

    fn spawn(ctx: &egui::Context, config: Arc<TermConfig>) -> SessionChannels {
        stream::open_session(ctx, move |s| forge_core::widgets::term::session(s, config))
    }

    pub fn status(&self) -> &TermStatus {
        &self.status
    }

    /// Programmatic input: send `s` to the tty as if typed.
    pub fn send_text(&mut self, s: &str) {
        self.send_bytes(s.as_bytes().to_vec());
    }

    /// Drop the session channels, ending the session (the engine kills its
    /// PTY/SSH connection). Status becomes [`TermStatus::Closed`] unless the
    /// session already ended with an exit code or error.
    pub fn disconnect(&mut self) {
        self.chan = None;
        if matches!(self.status, TermStatus::Connecting | TermStatus::Ready) {
            self.status = TermStatus::Closed;
        }
    }

    /// Re-open a finished session with the retained start parameters (shell
    /// config, or SSH host/credentials) on a fresh screen.
    pub fn restart(&mut self, ctx: &egui::Context) {
        self.chan = Some(TermState::spawn(ctx, self.spec.config.clone()));
        self.parser = vt100::Parser::new(self.grid.1.max(2), self.grid.0.max(2), SCROLLBACK);
        self.status = TermStatus::Connecting;
        self.started = false;
        self.pending_resize = None;
        self.scroll_accum = 0.0;
        self.last_mouse_cell = None;
    }

    /// Drain frames from the engine: tty bytes into the parser, control
    /// frames into status transitions. Called at the top of every `show()`.
    fn pump(&mut self) {
        let Some(chan) = &mut self.chan else { return };
        loop {
            match chan.rx.try_recv() {
                Ok(WidgetMsg::Binary(bytes)) => self.parser.process(&bytes),
                Ok(WidgetMsg::Text(text)) => match serde_json::from_str::<TermServerMsg>(&text) {
                    Ok(TermServerMsg::Ready) => self.status = TermStatus::Ready,
                    Ok(TermServerMsg::Exit { code }) => self.status = TermStatus::Exited(code),
                    Ok(TermServerMsg::Error { message }) => {
                        self.status = TermStatus::Error(message)
                    }
                    Err(_) => tracing::warn!("ignoring malformed term control frame"),
                },
                Ok(WidgetMsg::Close) | Err(TryRecvError::Disconnected) => {
                    if matches!(self.status, TermStatus::Connecting | TermStatus::Ready) {
                        self.status = TermStatus::Closed;
                    }
                    self.chan = None;
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    /// First call sends `start` with the measured grid; later calls debounce
    /// grid changes into `parser.set_size` + a `resize` frame.
    fn sync_grid(&mut self, ctx: &egui::Context, cols: u16, rows: u16, now: f64) {
        if !self.started {
            self.parser.set_size(rows, cols);
            self.grid = (cols, rows);
            let start = TermClientMsg::Start {
                mode: self.spec.mode,
                host: self.spec.host.clone(),
                port: self.spec.port,
                username: self.spec.username.clone(),
                password: self.spec.password.clone(),
                cols,
                rows,
            };
            // A failed send (channel gone) leaves `started` false, but the
            // session is over anyway — pump() reports Closed.
            self.started = self.send_ctrl(&start);
            return;
        }
        if (cols, rows) == self.grid {
            self.pending_resize = None;
            return;
        }
        match self.pending_resize {
            Some((target, since)) if target == (cols, rows) => {
                let elapsed = now - since;
                if elapsed >= RESIZE_DEBOUNCE {
                    self.parser.set_size(rows, cols);
                    self.grid = (cols, rows);
                    self.pending_resize = None;
                    self.send_ctrl(&TermClientMsg::Resize { cols, rows });
                } else {
                    // Wake up when the debounce window closes.
                    ctx.request_repaint_after(Duration::from_secs_f64(RESIZE_DEBOUNCE - elapsed));
                }
            }
            _ => {
                self.pending_resize = Some(((cols, rows), now));
                ctx.request_repaint_after(Duration::from_secs_f64(RESIZE_DEBOUNCE));
            }
        }
    }

    fn send_bytes(&mut self, bytes: Vec<u8>) -> bool {
        self.send_msg(WidgetMsg::Binary(bytes))
    }

    fn send_ctrl(&mut self, msg: &TermClientMsg) -> bool {
        let text = serde_json::to_string(msg).expect("TermClientMsg serializes");
        self.send_msg(WidgetMsg::Text(text))
    }

    /// UI-thread send: `try_send` only. A full channel means the engine is
    /// wedged behind backpressure — drop the frame and warn.
    fn send_msg(&mut self, msg: WidgetMsg) -> bool {
        let Some(chan) = &self.chan else { return false };
        match chan.tx.try_send(msg) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                tracing::warn!("terminal session channel full; dropping input frame");
                false
            }
            Err(TrySendError::Closed(_)) => false,
        }
    }
}

/// The terminal view: a bordered well filled with the vt100 grid. Builder +
/// `show(ui, &mut TermState)`, like every Forge widget.
#[derive(Clone, Copy, Debug)]
pub struct Terminal {
    rows: u16,
    font_size: Option<f32>,
}

impl Default for Terminal {
    fn default() -> Terminal {
        Terminal {
            rows: 24,
            font_size: None,
        }
    }
}

/// Per-show glyph/grid metrics.
struct Metrics {
    mono: FontId,
    mono_bold: FontId,
    cell_w: f32,
    cell_h: f32,
    cols: u16,
    rows: u16,
}

impl Terminal {
    pub fn new() -> Terminal {
        Terminal::default()
    }

    /// Well height in grid rows (default 24). Width always fills the
    /// available space; columns follow from the glyph width.
    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = rows.max(2);
        self
    }

    /// Mono font size in points (default: the theme's base type size).
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn show(self, ui: &mut Ui, state: &mut TermState) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        state.pump();

        let font_size = self.font_size.unwrap_or(t.type_scale.base);
        let mono = t.mono(font_size);
        let bold_family = egui::FontFamily::Name(MONO_BOLD_FAMILY.into());
        let mono_bold = if ui
            .ctx()
            .fonts(|f| f.definitions().families.contains_key(&bold_family))
        {
            FontId::new(font_size, bold_family)
        } else {
            mono.clone()
        };
        let (cell_w, cell_h) = ui
            .ctx()
            .fonts_mut(|f| (f.glyph_width(&mono, ' '), f.row_height(&mono)));
        // Guard degenerate metrics (glyph_width is 0.0 for a missing font).
        let cell_w = cell_w.max(1.0);
        let cell_h = cell_h.max(1.0);

        let width = ui.available_width().max(cell_w * 8.0 + PAD * 2.0);
        let height = self.rows as f32 * cell_h + PAD * 2.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "terminal"));

        let cols = (((rect.width() - PAD * 2.0) / cell_w) as i32).clamp(2, 1000) as u16;
        let rows = (((rect.height() - PAD * 2.0) / cell_h) as i32).clamp(2, 1000) as u16;
        let now = ui.input(|i| i.time);
        state.sync_grid(ui.ctx(), cols, rows, now);

        if response.clicked() {
            response.request_focus();
        }
        let focused = response.has_focus();

        let origin = rect.min + Vec2::splat(PAD);
        let mut outcome = Outcome::Ignored;
        if focused {
            if self.handle_input(ui, state, &response, origin, cell_w, cell_h) {
                outcome = Outcome::Consumed;
            }
        } else {
            state.scroll_accum = 0.0;
            state.last_mouse_cell = None;
        }
        // Re-read after input: Ctrl+Shift+Q surrenders focus this frame.
        let focused = response.has_focus();

        if ui.is_rect_visible(rect) {
            let metrics = Metrics {
                mono,
                mono_bold,
                cell_w,
                cell_h,
                cols,
                rows,
            };
            paint(ui, &t, state, rect, &metrics, focused);
        }

        ForgeResponse::new(response, outcome)
    }

    /// Encode this frame's captured events into tty bytes; returns whether
    /// anything was sent.
    fn handle_input(
        &self,
        ui: &mut Ui,
        state: &mut TermState,
        response: &egui::Response,
        origin: egui::Pos2,
        cell_w: f32,
        cell_h: f32,
    ) -> bool {
        // Keep Tab/arrows/Esc on the terminal instead of moving focus.
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

        let (app_cursor, bracketed, alternate, mouse_mode, mouse_encoding) = {
            let s = state.parser.screen();
            (
                s.application_cursor(),
                s.bracketed_paste(),
                s.alternate_screen(),
                s.mouse_protocol_mode(),
                s.mouse_protocol_encoding(),
            )
        };
        // Only forward pointer events when the running program asked for mouse
        // tracking; otherwise the wheel keeps its arrow-key scrollback shim.
        let mouse_on = mouse_mode != vt100::MouseProtocolMode::None;
        let any_down = ui.input(|i| i.pointer.any_down());

        let mut bytes = Vec::new();
        let mut saw_modified_key = false;
        let mut release = false;
        for event in ui.input(|i| i.events.clone()) {
            match event {
                // The capture-escape chord — never forwarded.
                egui::Event::Key {
                    key: Key::Q,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.ctrl && modifiers.shift => release = true,
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if let Some(seq) = encode_key(key, modifiers, app_cursor) {
                        if modifiers.ctrl || modifiers.alt {
                            // Some platforms also emit a Text event for the
                            // chord (e.g. Alt+x) — suppress it this frame.
                            saw_modified_key = true;
                        }
                        bytes.extend_from_slice(&seq);
                    }
                }
                egui::Event::Text(text) if !saw_modified_key => {
                    bytes.extend_from_slice(text.as_bytes());
                }
                egui::Event::Paste(text) => {
                    if bracketed {
                        bytes.extend_from_slice(b"\x1b[200~");
                        bytes.extend_from_slice(text.as_bytes());
                        bytes.extend_from_slice(b"\x1b[201~");
                    } else {
                        bytes.extend_from_slice(text.as_bytes());
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                } if mouse_on => {
                    if let Some(base) = button_base(button) {
                        if mode_allows(mouse_mode, base, false, !pressed) {
                            let (col, row) = cell_at(pos, origin, cell_w, cell_h, state.grid);
                            state.last_mouse_cell = Some((col, row));
                            bytes.extend_from_slice(&encode_mouse(
                                mouse_encoding,
                                base,
                                false,
                                !pressed,
                                col,
                                row,
                                modifiers.shift,
                                modifiers.alt,
                                modifiers.ctrl,
                            ));
                        }
                    }
                }
                egui::Event::PointerMoved(pos) if mouse_on => {
                    // A held button makes this a drag (button id in the low
                    // bits — egui doesn't say which, so use left per xterm
                    // convention); no button makes it bare motion (id "none").
                    let base = if any_down { 0 } else { BUTTON_NONE };
                    let (col, row) = cell_at(pos, origin, cell_w, cell_h, state.grid);
                    if mode_allows(mouse_mode, base, true, false)
                        && state.last_mouse_cell != Some((col, row))
                    {
                        state.last_mouse_cell = Some((col, row));
                        bytes.extend_from_slice(&encode_mouse(
                            mouse_encoding,
                            base,
                            true,
                            false,
                            col,
                            row,
                            false,
                            false,
                            false,
                        ));
                    }
                }
                _ => {}
            }
        }

        // Wheel: when the app tracks the mouse, report it as wheel buttons on
        // the hovered cell; otherwise keep the arrow-key scrollback shim for
        // alternate-screen apps (less/vim/htop). The primary screen has no
        // scrollback in v1 (forge-tui parity).
        if response.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if mouse_on {
                if dy != 0.0 {
                    state.scroll_accum += dy;
                    let lines = (state.scroll_accum / cell_h) as i32;
                    if lines != 0 {
                        state.scroll_accum -= lines as f32 * cell_h;
                        let base = if lines > 0 { WHEEL_UP } else { WHEEL_DOWN };
                        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                            let (col, row) = cell_at(pos, origin, cell_w, cell_h, state.grid);
                            for _ in 0..lines.unsigned_abs() {
                                bytes.extend_from_slice(&encode_mouse(
                                    mouse_encoding,
                                    base,
                                    false,
                                    false,
                                    col,
                                    row,
                                    false,
                                    false,
                                    false,
                                ));
                            }
                        }
                    }
                }
            } else if alternate && dy != 0.0 {
                state.scroll_accum += dy;
                let lines = (state.scroll_accum / cell_h) as i32;
                if lines != 0 {
                    state.scroll_accum -= lines as f32 * cell_h;
                    let seq: &[u8] = match (lines > 0, app_cursor) {
                        (true, true) => b"\x1bOA",
                        (true, false) => b"\x1b[A",
                        (false, true) => b"\x1bOB",
                        (false, false) => b"\x1b[B",
                    };
                    for _ in 0..lines.unsigned_abs() {
                        bytes.extend_from_slice(seq);
                    }
                }
            } else if !alternate {
                state.scroll_accum = 0.0;
            }
        }

        let sent = !bytes.is_empty() && state.send_bytes(bytes);
        if release {
            response.surrender_focus();
        }
        sent
    }
}

// xterm mouse button codes (the low bits of `cb`, before modifier/motion bits).
const BUTTON_NONE: u16 = 3; // no button held (bare motion, X10 release)
const WHEEL_UP: u16 = 64;
const WHEEL_DOWN: u16 = 65;

fn button_base(b: egui::PointerButton) -> Option<u16> {
    match b {
        egui::PointerButton::Primary => Some(0),
        egui::PointerButton::Middle => Some(1),
        egui::PointerButton::Secondary => Some(2),
        _ => None,
    }
}

/// Pixel position → 0-based cell, clamped to the grid.
fn cell_at(
    pos: egui::Pos2,
    origin: egui::Pos2,
    cell_w: f32,
    cell_h: f32,
    grid: (u16, u16),
) -> (u16, u16) {
    let col = ((pos.x - origin.x) / cell_w)
        .floor()
        .clamp(0.0, grid.0.saturating_sub(1) as f32) as u16;
    let row = ((pos.y - origin.y) / cell_h)
        .floor()
        .clamp(0.0, grid.1.saturating_sub(1) as f32) as u16;
    (col, row)
}

/// Does the active tracking mode want to report this event? Mirrors xterm:
/// `Press` reports button-down + wheel only; `PressRelease` adds releases;
/// `ButtonMotion` adds drags (motion with a button); `AnyMotion` adds bare
/// motion. Wheel is a "press" and is reported by every non-`None` mode.
fn mode_allows(mode: vt100::MouseProtocolMode, button: u16, motion: bool, release: bool) -> bool {
    use vt100::MouseProtocolMode as M;
    let wheel = button >= WHEEL_UP;
    match mode {
        M::None => false,
        M::Press => !release && (!motion || wheel),
        M::PressRelease => !motion || wheel,
        M::ButtonMotion => wheel || !(motion && button == BUTTON_NONE),
        M::AnyMotion => true,
    }
}

/// Build an xterm mouse report in the screen's active encoding. `col`/`row` are
/// 0-based cells; the wire format is 1-based. `button` is the base code
/// (0/1/2, [`BUTTON_NONE`], or a `WHEEL_*`); modifier and motion bits are added
/// here.
#[allow(clippy::too_many_arguments)]
fn encode_mouse(
    encoding: vt100::MouseProtocolEncoding,
    button: u16,
    motion: bool,
    release: bool,
    col: u16,
    row: u16,
    shift: bool,
    alt: bool,
    ctrl: bool,
) -> Vec<u8> {
    let mut cb = button;
    if shift {
        cb += 4;
    }
    if alt {
        cb += 8;
    }
    if ctrl {
        cb += 16;
    }
    if motion {
        cb += 32;
    }
    match encoding {
        vt100::MouseProtocolEncoding::Sgr => {
            let final_byte = if release { 'm' } else { 'M' };
            format!("\x1b[<{};{};{}{}", cb, col + 1, row + 1, final_byte).into_bytes()
        }
        vt100::MouseProtocolEncoding::Utf8 => {
            // Release drops the button id to "none" (X10 has no release byte).
            let cb = if release {
                (cb & !0b11) | BUTTON_NONE
            } else {
                cb
            };
            let mut out = vec![0x1b, b'[', b'M'];
            push_utf8(&mut out, cb + 32);
            push_utf8(&mut out, col + 1 + 32);
            push_utf8(&mut out, row + 1 + 32);
            out
        }
        // Default (X10): single printable byte per field, saturating at 255.
        _ => {
            let cb = if release {
                (cb & !0b11) | BUTTON_NONE
            } else {
                cb
            };
            vec![
                0x1b,
                b'[',
                b'M',
                (cb + 32).min(255) as u8,
                (col + 1 + 32).min(255) as u8,
                (row + 1 + 32).min(255) as u8,
            ]
        }
    }
}

/// Append `v` as a UTF-8 code point (the `?1005` encoding widens coords past
/// the 223-cell wall the single-byte form hits).
fn push_utf8(out: &mut Vec<u8>, v: u16) {
    let ch = char::from_u32(v as u32).unwrap_or('\u{fffd}');
    let mut buf = [0u8; 4];
    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
}

/// Paint the well, grid, cursor, capture badge, and status overlays.
fn paint(ui: &Ui, t: &Theme, state: &TermState, rect: Rect, m: &Metrics, focused: bool) {
    let radius = CornerRadius::same(t.radius.md as u8);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, t.bg[1]);
    let border = if focused {
        t.accent.base
    } else {
        t.border.default
    };
    painter.rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);

    let grid = painter.with_clip_rect(rect.shrink(1.0));
    let origin = rect.min + Vec2::splat(PAD);
    let screen = state.parser.screen();
    let (srows, scols) = screen.size();
    let rows = m.rows.min(srows);
    let cols = m.cols.min(scols);

    for row in 0..rows {
        let y = origin.y + row as f32 * m.cell_h;
        let mut job = LayoutJob {
            break_on_newline: false,
            ..Default::default()
        };
        let mut run = String::new();
        let mut run_fmt: Option<TextFormat> = None;
        // Background run: (start col, end col exclusive, color).
        let mut bg_run: Option<(u16, u16, Color32)> = None;
        let flush_bg = |bg_run: &mut Option<(u16, u16, Color32)>| {
            if let Some((start, end, color)) = bg_run.take() {
                let r = Rect::from_min_size(
                    egui::pos2(origin.x + start as f32 * m.cell_w, y),
                    Vec2::new((end - start) as f32 * m.cell_w, m.cell_h),
                );
                grid.rect_filled(r, 0.0, color);
            }
        };

        for col in 0..cols {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            if cell.is_wide_continuation() {
                // The wide glyph in the previous cell spans this one; only
                // its background run extends.
                if let Some((_, end, _)) = &mut bg_run {
                    if *end == col {
                        *end = col + 1;
                    }
                }
                continue;
            }

            let mut fg = palette::fg(cell.fgcolor(), t);
            let mut bg = palette::bg(cell.bgcolor(), t);
            if cell.inverse() {
                let inv_fg = bg.unwrap_or(t.bg[1]);
                bg = Some(fg);
                fg = inv_fg;
            }

            match bg {
                Some(color) => match &mut bg_run {
                    Some((_, end, run_color)) if *run_color == color && *end == col => {
                        *end = col + 1;
                    }
                    _ => {
                        flush_bg(&mut bg_run);
                        bg_run = Some((col, col + 1, color));
                    }
                },
                None => flush_bg(&mut bg_run),
            }

            let contents = cell.contents();
            let mut fmt = TextFormat {
                font_id: if cell.bold() {
                    m.mono_bold.clone()
                } else {
                    m.mono.clone()
                },
                color: fg,
                italics: cell.italic(),
                ..Default::default()
            };
            if cell.underline() {
                fmt.underline = Stroke::new(1.0, fg);
            }
            if run_fmt.as_ref() != Some(&fmt) {
                if let Some(prev) = run_fmt.take() {
                    if !run.is_empty() {
                        job.append(&run, 0.0, prev);
                        run.clear();
                    }
                }
                run_fmt = Some(fmt);
            }
            if contents.is_empty() {
                run.push(' ');
            } else {
                run.push_str(&contents);
            }
        }
        flush_bg(&mut bg_run);
        if let Some(fmt) = run_fmt.take() {
            if !run.is_empty() {
                job.append(&run, 0.0, fmt);
            }
        }
        if !job.is_empty() {
            let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
            grid.galley(egui::pos2(origin.x, y), galley, t.fg[0]);
        }
    }

    // Blinking block cursor with an inverted glyph, while captured.
    if focused && state.status == TermStatus::Ready && !screen.hide_cursor() {
        let (crow, ccol) = screen.cursor_position();
        if crow < rows && ccol < cols {
            let time = ui.input(|i| i.time);
            if (time / BLINK) as i64 % 2 == 0 {
                let cursor = Rect::from_min_size(
                    egui::pos2(
                        origin.x + ccol as f32 * m.cell_w,
                        origin.y + crow as f32 * m.cell_h,
                    ),
                    Vec2::new(m.cell_w, m.cell_h),
                );
                grid.rect_filled(cursor, 0.0, t.fg[0]);
                if let Some(cell) = screen.cell(crow, ccol) {
                    let ch = cell.contents();
                    if !ch.is_empty() {
                        grid.text(cursor.min, Align2::LEFT_TOP, ch, m.mono.clone(), t.bg[1]);
                    }
                }
            }
            let until_flip = BLINK - (time % BLINK);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(until_flip.max(0.016)));
        }
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
        TermStatus::Ready => {}
        TermStatus::Connecting => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "connecting…",
                t.mono(t.type_scale.sm),
                t.fg[2],
            );
        }
        TermStatus::Exited(code) => {
            end_overlay(
                painter,
                rect,
                t,
                radius,
                &format!("process exited (code {code})"),
            );
        }
        TermStatus::Closed => end_overlay(painter, rect, t, radius, "session closed"),
        TermStatus::Error(message) => {
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
                egui::pos2(banner.min.x + PAD, banner.center().y),
                Align2::LEFT_CENTER,
                message,
                t.mono(t.type_scale.xs),
                t.danger.fg,
            );
        }
    }
}

/// Dim scrim + centered message for finished sessions.
fn end_overlay(painter: &egui::Painter, rect: Rect, t: &Theme, radius: CornerRadius, msg: &str) {
    painter.rect_filled(rect, radius, scrim(t));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        msg,
        t.mono(t.type_scale.sm),
        t.fg[1],
    );
}

/// Encode a non-text key press as xterm bytes. Printable characters arrive as
/// `Event::Text` and are NOT encoded here — only editing/navigation keys and
/// ctrl/alt chords.
fn encode_key(key: Key, modifiers: egui::Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    // Ctrl+alpha → control byte (Ctrl+C = 0x03). No Text event accompanies
    // these on any platform we target.
    if modifiers.ctrl && !modifiers.alt {
        if let Some(c) = key_char(key) {
            if c.is_ascii_alphabetic() {
                return Some(vec![(c.to_ascii_uppercase() as u8) & 0x1f]);
            }
        }
    }
    // Alt+char → ESC prefix (readline meta).
    if modifiers.alt && !modifiers.ctrl {
        if let Some(c) = key_char(key) {
            return Some(vec![0x1b, c as u8]);
        }
    }
    let arrow = |ch: u8| -> Vec<u8> {
        if app_cursor {
            vec![0x1b, b'O', ch]
        } else {
            vec![0x1b, b'[', ch]
        }
    };
    Some(match key {
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![0x1b],
        Key::ArrowUp => arrow(b'A'),
        Key::ArrowDown => arrow(b'B'),
        Key::ArrowRight => arrow(b'C'),
        Key::ArrowLeft => arrow(b'D'),
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::F1 => b"\x1bOP".to_vec(),
        Key::F2 => b"\x1bOQ".to_vec(),
        Key::F3 => b"\x1bOR".to_vec(),
        Key::F4 => b"\x1bOS".to_vec(),
        Key::F5 => b"\x1b[15~".to_vec(),
        Key::F6 => b"\x1b[17~".to_vec(),
        Key::F7 => b"\x1b[18~".to_vec(),
        Key::F8 => b"\x1b[19~".to_vec(),
        Key::F9 => b"\x1b[20~".to_vec(),
        Key::F10 => b"\x1b[21~".to_vec(),
        Key::F11 => b"\x1b[23~".to_vec(),
        Key::F12 => b"\x1b[24~".to_vec(),
        _ => return None,
    })
}

/// The single ASCII character a key produces unmodified, lowercased —
/// `Key::A` → `'a'`, `Key::Num1` → `'1'`, `Key::Slash` → `'/'`.
fn key_char(key: Key) -> Option<char> {
    let name = key.symbol_or_name();
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii() => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

/// vt100 → egui colors, derived from the theme so terminal output sits on the
/// Forge palette (the port of forge-tui's `map_color`, which deferred to the
/// host terminal's ANSI palette — here we ARE the terminal).
mod palette {
    use crate::theme::{blend, Theme};
    use egui::Color32;

    /// Foreground: default ink is the theme's primary text.
    pub(super) fn fg(c: vt100::Color, t: &Theme) -> Color32 {
        match c {
            vt100::Color::Default => t.fg[0],
            vt100::Color::Idx(i) => indexed(i, t),
            vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        }
    }

    /// Background: `None` = the terminal well shows through (no rect).
    pub(super) fn bg(c: vt100::Color, t: &Theme) -> Option<Color32> {
        match c {
            vt100::Color::Default => None,
            vt100::Color::Idx(i) => Some(indexed(i, t)),
            vt100::Color::Rgb(r, g, b) => Some(Color32::from_rgb(r, g, b)),
        }
    }

    /// The 256-color table: ANSI 16 from theme tokens, then the standard
    /// 6×6×6 cube and gray ramp (xterm component values).
    pub(super) fn indexed(i: u8, t: &Theme) -> Color32 {
        match i {
            0 => t.bg[3],                                  // black
            1 => t.danger.base,                            // red
            2 => t.success.base,                           // green
            3 => t.warning.base,                           // yellow
            4 => t.accent.base,                            // blue
            5 => blend(t.danger.base, t.accent.base, 0.5), // magenta (violet blend)
            6 => t.info.base,                              // cyan
            7 => t.fg[1],                                  // white
            8 => t.fg[2],                                  // bright black
            9 => t.danger.fg,                              // bright red
            10 => t.success.fg,                            // bright green
            11 => t.warning.fg,                            // bright yellow
            12 => t.accent.fg,                             // bright blue
            13 => blend(t.danger.fg, t.accent.fg, 0.5),    // bright magenta
            14 => t.info.fg,                               // bright cyan
            15 => t.fg[0],                                 // bright white
            16..=231 => {
                let n = i - 16;
                let comp = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                Color32::from_rgb(comp(n / 36), comp((n % 36) / 6), comp(n % 6))
            }
            232..=255 => {
                let v = 8 + 10 * (i - 232);
                Color32::from_rgb(v, v, v)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn ansi16_maps_to_theme_tokens() {
        let t = Theme::dark();
        assert_eq!(palette::indexed(0, &t), t.bg[3]);
        assert_eq!(palette::indexed(1, &t), t.danger.base);
        assert_eq!(palette::indexed(2, &t), t.success.base);
        assert_eq!(palette::indexed(3, &t), t.warning.base);
        assert_eq!(palette::indexed(4, &t), t.accent.base);
        assert_eq!(palette::indexed(6, &t), t.info.base);
        assert_eq!(palette::indexed(7, &t), t.fg[1]);
        assert_eq!(palette::indexed(9, &t), t.danger.fg);
        assert_eq!(palette::indexed(15, &t), t.fg[0]);
        // Default fg is primary text; default bg is transparent (the well).
        assert_eq!(palette::fg(vt100::Color::Default, &t), t.fg[0]);
        assert_eq!(palette::bg(vt100::Color::Default, &t), None);
        assert_eq!(
            palette::fg(vt100::Color::Rgb(1, 2, 3), &t),
            egui::Color32::from_rgb(1, 2, 3)
        );
    }

    #[test]
    fn color_cube_and_gray_ramp_math() {
        let t = Theme::dark();
        // Cube corners and a mid gray, per the xterm component table.
        assert_eq!(palette::indexed(16, &t), egui::Color32::from_rgb(0, 0, 0));
        assert_eq!(palette::indexed(21, &t), egui::Color32::from_rgb(0, 0, 255));
        assert_eq!(
            palette::indexed(231, &t),
            egui::Color32::from_rgb(255, 255, 255)
        );
        assert_eq!(
            palette::indexed(244, &t),
            egui::Color32::from_rgb(128, 128, 128)
        );
        assert_eq!(
            palette::indexed(255, &t),
            egui::Color32::from_rgb(238, 238, 238)
        );
    }

    #[test]
    fn xterm_key_encoding() {
        let none = egui::Modifiers::NONE;
        assert_eq!(encode_key(Key::Enter, none, false), Some(b"\r".to_vec()));
        assert_eq!(encode_key(Key::Backspace, none, false), Some(vec![0x7f]));
        assert_eq!(
            encode_key(Key::ArrowUp, none, false),
            Some(b"\x1b[A".to_vec())
        );
        // Application cursor mode switches arrows to SS3.
        assert_eq!(
            encode_key(Key::ArrowUp, none, true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            encode_key(Key::Delete, none, false),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(encode_key(Key::F5, none, false), Some(b"\x1b[15~".to_vec()));
        // Ctrl+C → ETX; Alt+X → ESC-prefixed meta.
        assert_eq!(
            encode_key(Key::C, egui::Modifiers::CTRL, false),
            Some(vec![0x03])
        );
        assert_eq!(
            encode_key(Key::X, egui::Modifiers::ALT, false),
            Some(vec![0x1b, b'x'])
        );
        // Plain printable keys are Text events, never encoded here.
        assert_eq!(encode_key(Key::A, none, false), None);
    }

    #[test]
    fn xterm_mouse_encoding() {
        use vt100::{MouseProtocolEncoding as Enc, MouseProtocolMode as Mode};
        let sgr = |button, motion, release, col, row| {
            String::from_utf8(encode_mouse(
                Enc::Sgr,
                button,
                motion,
                release,
                col,
                row,
                false,
                false,
                false,
            ))
            .unwrap()
        };
        // Left press/release at 0-based cell → 1-based coords, M vs m final.
        assert_eq!(sgr(0, false, false, 0, 0), "\x1b[<0;1;1M");
        assert_eq!(sgr(0, false, true, 4, 2), "\x1b[<0;5;3m");
        // Right-button drag sets the motion bit (+32).
        assert_eq!(sgr(2, true, false, 9, 1), "\x1b[<34;10;2M");
        // Wheel up is a press with cb 64.
        assert_eq!(sgr(WHEEL_UP, false, false, 3, 3), "\x1b[<64;4;4M");
        // Ctrl+Shift left press → cb 20.
        assert_eq!(
            String::from_utf8(encode_mouse(
                Enc::Sgr,
                0,
                false,
                false,
                0,
                0,
                true,
                false,
                true
            ))
            .unwrap(),
            "\x1b[<20;1;1M"
        );
        // X10 default bytes: ESC [ M then 32+cb, 32+col+1, 32+row+1; release
        // drops the button id to "none" (3).
        assert_eq!(
            encode_mouse(Enc::Default, 0, false, false, 0, 0, false, false, false),
            vec![0x1b, b'[', b'M', 32, 33, 33]
        );
        assert_eq!(
            encode_mouse(Enc::Default, 2, false, true, 0, 0, false, false, false),
            vec![0x1b, b'[', b'M', 32 + 3, 33, 33]
        );

        // Mode gating (mirrors forge-tui).
        assert!(mode_allows(Mode::Press, 0, false, false)); // down
        assert!(mode_allows(Mode::Press, WHEEL_UP, false, false)); // wheel
        assert!(!mode_allows(Mode::Press, 0, false, true)); // up
        assert!(!mode_allows(Mode::Press, 0, true, false)); // drag
        assert!(mode_allows(Mode::PressRelease, 0, false, true)); // up
        assert!(!mode_allows(Mode::PressRelease, 0, true, false)); // move
        assert!(mode_allows(Mode::ButtonMotion, 0, true, false)); // drag
        assert!(!mode_allows(Mode::ButtonMotion, BUTTON_NONE, true, false)); // bare move
        assert!(mode_allows(Mode::AnyMotion, BUTTON_NONE, true, false)); // bare move
        assert!(!mode_allows(Mode::None, 0, false, false)); // nothing
    }

    /// End-to-end over the real engine + lazy runtime: start → ready →
    /// printf round-trip → debounced resize (stty reflects it) → disconnect.
    #[test]
    fn local_terminal_end_to_end() {
        let deadline = Instant::now() + Duration::from_secs(15);
        let ctx = egui::Context::default();
        let mut state = TermState::local_with(
            &ctx,
            TermConfig {
                shell: Some("/bin/sh".into()),
                ..TermConfig::default()
            },
        );

        fn frame(ctx: &egui::Context, state: &mut TermState, width: f32) {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 460.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run_ui(raw, |ui| {
                let _ = Terminal::new().rows(24).show(ui, state);
            });
        }

        let pump_until =
            |state: &mut TermState, width: f32, done: &mut dyn FnMut(&TermState) -> bool| loop {
                frame(&ctx, state, width);
                if done(state) {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out; status={:?} screen={:?}",
                    state.status,
                    state.parser.screen().contents()
                );
                std::thread::sleep(Duration::from_millis(25));
            };

        // The first frame sends `start`; the engine answers `ready`.
        pump_until(&mut state, 760.0, &mut |s| s.status == TermStatus::Ready);
        let cols_before = state.grid.0;
        assert!(cols_before > 8, "measured grid too small: {cols_before}");

        // printf assembles the marker so the local echo of the typed command
        // can't satisfy the assertion.
        state.send_text("printf 'forge%s\\n' -w2-ok\r");
        pump_until(&mut state, 760.0, &mut |s| {
            s.parser.screen().contents().contains("forge-w2-ok")
        });

        // Shrink the window: after the debounce the grid re-measures and the
        // engine's PTY follows (stty reports the new size).
        pump_until(&mut state, 460.0, &mut |s| {
            s.grid.0 != cols_before && s.pending_resize.is_none()
        });
        let (cols, rows) = state.grid;
        assert!(cols < cols_before);
        state.send_text("stty size\r");
        let expect = format!("{rows} {cols}");
        pump_until(&mut state, 460.0, &mut |s| {
            s.parser.screen().contents().contains(&expect)
        });

        state.disconnect();
        assert_eq!(*state.status(), TermStatus::Closed);
    }

    /// SSH e2e against a disposable container:
    /// `docker run --rm -d --name forge-ssh-test -p 127.0.0.1:2222:2222 \
    ///    -e PASSWORD_ACCESS=true -e USER_NAME=forge -e USER_PASSWORD=forge \
    ///    lscr.io/linuxserver/openssh-server`
    /// then `cargo test -p forge-egui --features term-ssh -- --ignored ssh_`.
    #[cfg(feature = "term-ssh")]
    #[test]
    #[ignore = "needs a live sshd on 127.0.0.1:2222 (see doc comment)"]
    fn ssh_terminal_end_to_end() {
        let deadline = Instant::now() + Duration::from_secs(20);
        let ctx = egui::Context::default();
        let mut state = TermState::ssh(
            &ctx,
            SshOptions {
                host: "127.0.0.1".into(),
                port: 2222,
                username: "forge".into(),
                password: "forge".into(),
            },
        );

        let pump_until = |state: &mut TermState, done: &mut dyn FnMut(&TermState) -> bool| loop {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(760.0, 460.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run_ui(raw, |ui| {
                let _ = Terminal::new().rows(24).show(ui, state);
            });
            if done(state) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out; status={:?} screen={:?}",
                state.status,
                state.parser.screen().contents()
            );
            std::thread::sleep(Duration::from_millis(25));
        };

        pump_until(&mut state, &mut |s| s.status == TermStatus::Ready);
        state.send_text("printf 'forge%s\\n' -ssh-ok\r");
        pump_until(&mut state, &mut |s| {
            s.parser.screen().contents().contains("forge-ssh-ok")
        });
        state.disconnect();
        assert_eq!(*state.status(), TermStatus::Closed);
    }

    /// Wrong password surfaces a clean Error status (no panic, no hang).
    #[cfg(feature = "term-ssh")]
    #[test]
    #[ignore = "needs a live sshd on 127.0.0.1:2222 (see doc comment)"]
    fn ssh_wrong_password_errors() {
        let deadline = Instant::now() + Duration::from_secs(20);
        let ctx = egui::Context::default();
        let mut state = TermState::ssh(
            &ctx,
            SshOptions {
                host: "127.0.0.1".into(),
                port: 2222,
                username: "forge".into(),
                password: "wrong".into(),
            },
        );
        loop {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(760.0, 460.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run_ui(raw, |ui| {
                let _ = Terminal::new().rows(24).show(ui, &mut state);
            });
            match &state.status {
                TermStatus::Error(_) | TermStatus::Closed => break,
                _ => {}
            }
            assert!(Instant::now() < deadline, "timed out awaiting auth error");
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            matches!(state.status, TermStatus::Error(_) | TermStatus::Closed),
            "expected error/closed, got {:?}",
            state.status
        );
    }
}
