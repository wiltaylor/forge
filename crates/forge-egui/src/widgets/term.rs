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
use forge_xterm::key::{self, CursorKeys, Key as XtermKey};
use forge_xterm::mouse::{
    self, MouseEncoding, MouseMode, MouseReport, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_NONE,
    BUTTON_RIGHT, WHEEL_DOWN, WHEEL_UP,
};
use forge_xterm::Modifiers;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{scrim, Surface, TextRole, Theme};
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
    /// grid changes into `parser.screen_mut().set_size` + a `resize` frame.
    fn sync_grid(&mut self, ctx: &egui::Context, cols: u16, rows: u16, now: f64) {
        if !self.started {
            self.parser.screen_mut().set_size(rows, cols);
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
                    self.parser.screen_mut().set_size(rows, cols);
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

        let (cursor, bracketed, alternate, mouse_mode, mouse_encoding) = {
            let s = state.parser.screen();
            (
                cursor_keys(s),
                s.bracketed_paste(),
                s.alternate_screen(),
                to_mouse_mode(s.mouse_protocol_mode()),
                to_mouse_encoding(s.mouse_protocol_encoding()),
            )
        };
        // Only forward pointer events when the running program asked for mouse
        // tracking; otherwise the wheel keeps its arrow-key scrollback shim.
        let mouse_on = mouse_mode != MouseMode::None;
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
                    if let Some(seq) = encode_key(key, modifiers, cursor) {
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
                        let (col, row) = cell_at(pos, origin, cell_w, cell_h, state.grid);
                        let report = MouseReport {
                            button: base,
                            motion: false,
                            release: !pressed,
                            col,
                            row,
                            modifiers: to_modifiers(modifiers),
                        };
                        if mouse::is_reported(&report, mouse_mode) {
                            state.last_mouse_cell = Some((col, row));
                            bytes.extend_from_slice(&mouse::encode(&report, mouse_encoding));
                        }
                    }
                }
                egui::Event::PointerMoved(pos) if mouse_on => {
                    // A held button makes this a drag (button id in the low
                    // bits — egui doesn't say which, so use left per xterm
                    // convention); no button makes it bare motion (id "none").
                    let base = if any_down { BUTTON_LEFT } else { BUTTON_NONE };
                    let (col, row) = cell_at(pos, origin, cell_w, cell_h, state.grid);
                    let report = MouseReport::drag(base, col, row);
                    if mouse::is_reported(&report, mouse_mode)
                        && state.last_mouse_cell != Some((col, row))
                    {
                        state.last_mouse_cell = Some((col, row));
                        bytes.extend_from_slice(&mouse::encode(&report, mouse_encoding));
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
                            let report = MouseReport::press(base, col, row);
                            if mouse::is_reported(&report, mouse_mode) {
                                let seq = mouse::encode(&report, mouse_encoding);
                                for _ in 0..lines.unsigned_abs() {
                                    bytes.extend_from_slice(&seq);
                                }
                            }
                        }
                    }
                }
            } else if alternate && dy != 0.0 {
                state.scroll_accum += dy;
                let lines = (state.scroll_accum / cell_h) as i32;
                if lines != 0 {
                    state.scroll_accum -= lines as f32 * cell_h;
                    let arrow = if lines > 0 { XtermKey::Up } else { XtermKey::Down };
                    let seq = key::encode(arrow, Modifiers::NONE, cursor)
                        .expect("the arrow keys encode in both cursor modes");
                    for _ in 0..lines.unsigned_abs() {
                        bytes.extend_from_slice(&seq);
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

/// egui's pointer buttons in the shared crate's button codes. `None` = no
/// xterm code for the extra buttons, so nothing is reported for them.
fn button_base(b: egui::PointerButton) -> Option<u16> {
    match b {
        egui::PointerButton::Primary => Some(BUTTON_LEFT),
        egui::PointerButton::Middle => Some(BUTTON_MIDDLE),
        egui::PointerButton::Secondary => Some(BUTTON_RIGHT),
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

/// The cursor-key mode the running program asked for (DECCKM `?1h`/`?1l`).
fn cursor_keys(screen: &vt100::Screen) -> CursorKeys {
    if screen.application_cursor() {
        CursorKeys::Application
    } else {
        CursorKeys::Normal
    }
}

/// egui's modifier set in the shared crate's vocabulary.
fn to_modifiers(m: egui::Modifiers) -> Modifiers {
    Modifiers {
        shift: m.shift,
        alt: m.alt,
        ctrl: m.ctrl,
    }
}

/// vt100's tracking-mode vocabulary in the shared crate's.
fn to_mouse_mode(mode: vt100::MouseProtocolMode) -> MouseMode {
    match mode {
        vt100::MouseProtocolMode::None => MouseMode::None,
        vt100::MouseProtocolMode::Press => MouseMode::Press,
        vt100::MouseProtocolMode::PressRelease => MouseMode::PressRelease,
        vt100::MouseProtocolMode::ButtonMotion => MouseMode::ButtonMotion,
        vt100::MouseProtocolMode::AnyMotion => MouseMode::AnyMotion,
    }
}

/// vt100's encoding vocabulary in the shared crate's.
fn to_mouse_encoding(encoding: vt100::MouseProtocolEncoding) -> MouseEncoding {
    match encoding {
        vt100::MouseProtocolEncoding::Default => MouseEncoding::Default,
        vt100::MouseProtocolEncoding::Utf8 => MouseEncoding::Utf8,
        vt100::MouseProtocolEncoding::Sgr => MouseEncoding::Sgr,
    }
}

/// Paint the well, grid, cursor, capture badge, and status overlays.
fn paint(ui: &Ui, t: &Theme, state: &TermState, rect: Rect, m: &Metrics, focused: bool) {
    let radius = CornerRadius::same(t.radius.md as u8);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, t.surface(Surface::Card));
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
                let inv_fg = bg.unwrap_or(t.surface(Surface::Card));
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
            grid.galley(egui::pos2(origin.x, y), galley, t.text(TextRole::Primary));
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
                grid.rect_filled(cursor, 0.0, t.text(TextRole::Primary));
                if let Some(cell) = screen.cell(crow, ccol) {
                    let ch = cell.contents();
                    if !ch.is_empty() {
                        grid.text(
                            cursor.min,
                            Align2::LEFT_TOP,
                            ch,
                            m.mono.clone(),
                            t.surface(Surface::Card),
                        );
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
        let galley = painter.layout_no_wrap(
            "▣ captured · Ctrl+Shift+Q releases".into(),
            font,
            t.text(TextRole::Tertiary),
        );
        let pad = Vec2::new(6.0, 3.0);
        let size = galley.size() + pad * 2.0;
        let chip = Rect::from_min_size(
            egui::pos2(rect.max.x - size.x - 6.0, rect.min.y + 6.0),
            size,
        );
        painter.rect_filled(
            chip,
            CornerRadius::same(t.radius.sm as u8),
            t.surface(Surface::Pressed),
        );
        painter.galley(chip.min + pad, galley, t.text(TextRole::Tertiary));
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
                t.text(TextRole::Tertiary),
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
        t.text(TextRole::Secondary),
    );
}

/// Encode a non-text key press as xterm bytes, through the shared table.
/// Printable characters arrive as `Event::Text` and are NOT encoded here —
/// only editing/navigation keys and ctrl/alt chords.
fn encode_key(key: Key, modifiers: egui::Modifiers, cursor: CursorKeys) -> Option<Vec<u8>> {
    let mods = to_modifiers(modifiers);
    key::encode(xterm_key(key, mods)?, mods, cursor)
}

/// egui's key vocabulary mapped onto the shared table's ([`XtermKey`]).
///
/// A character key resolves only while Ctrl or Alt is held: a plain (or
/// shifted) press arrives as `Event::Text`, which already carries the
/// character, so asking the table too would send it twice. The keys the
/// fallthrough leaves unresolved are pinned by the totality test.
fn xterm_key(key: Key, modifiers: Modifiers) -> Option<XtermKey> {
    Some(match key {
        Key::Enter => XtermKey::Enter,
        Key::Backspace => XtermKey::Backspace,
        Key::Tab => XtermKey::Tab,
        Key::Escape => XtermKey::Escape,
        Key::ArrowUp => XtermKey::Up,
        Key::ArrowDown => XtermKey::Down,
        Key::ArrowRight => XtermKey::Right,
        Key::ArrowLeft => XtermKey::Left,
        Key::Home => XtermKey::Home,
        Key::End => XtermKey::End,
        Key::PageUp => XtermKey::PageUp,
        Key::PageDown => XtermKey::PageDown,
        Key::Delete => XtermKey::Delete,
        Key::Insert => XtermKey::Insert,
        Key::F1 => XtermKey::Function(1),
        Key::F2 => XtermKey::Function(2),
        Key::F3 => XtermKey::Function(3),
        Key::F4 => XtermKey::Function(4),
        Key::F5 => XtermKey::Function(5),
        Key::F6 => XtermKey::Function(6),
        Key::F7 => XtermKey::Function(7),
        Key::F8 => XtermKey::Function(8),
        Key::F9 => XtermKey::Function(9),
        Key::F10 => XtermKey::Function(10),
        Key::F11 => XtermKey::Function(11),
        Key::F12 => XtermKey::Function(12),
        _ if modifiers.ctrl || modifiers.alt => XtermKey::Char(key_char(key)?),
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
    use crate::theme::{blend, Surface, TextRole, Theme};
    use egui::Color32;

    /// Foreground: default ink is the theme's primary text.
    pub(super) fn fg(c: vt100::Color, t: &Theme) -> Color32 {
        match c {
            vt100::Color::Default => t.text(TextRole::Primary),
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
            0 => t.surface(Surface::Pressed),              // black
            1 => t.danger.base,                            // red
            2 => t.success.base,                           // green
            3 => t.warning.base,                           // yellow
            4 => t.accent.base,                            // blue
            5 => blend(t.danger.base, t.accent.base, 0.5), // magenta (violet blend)
            6 => t.info.base,                              // cyan
            7 => t.text(TextRole::Secondary),              // white
            8 => t.text(TextRole::Tertiary),               // bright black
            9 => t.danger.fg,                              // bright red
            10 => t.success.fg,                            // bright green
            11 => t.warning.fg,                            // bright yellow
            12 => t.accent.fg,                             // bright blue
            13 => blend(t.danger.fg, t.accent.fg, 0.5),    // bright magenta
            14 => t.info.fg,                               // bright cyan
            15 => t.text(TextRole::Primary),               // bright white
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
        assert_eq!(palette::indexed(0, &t), t.surface(Surface::Pressed));
        assert_eq!(palette::indexed(1, &t), t.danger.base);
        assert_eq!(palette::indexed(2, &t), t.success.base);
        assert_eq!(palette::indexed(3, &t), t.warning.base);
        assert_eq!(palette::indexed(4, &t), t.accent.base);
        assert_eq!(palette::indexed(6, &t), t.info.base);
        assert_eq!(palette::indexed(7, &t), t.text(TextRole::Secondary));
        assert_eq!(palette::indexed(9, &t), t.danger.fg);
        assert_eq!(palette::indexed(15, &t), t.text(TextRole::Primary));
        // Default fg is primary text; default bg is transparent (the well).
        assert_eq!(
            palette::fg(vt100::Color::Default, &t),
            t.text(TextRole::Primary)
        );
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

    // The encoding tests (bytes per event, mode gating) live in forge-xterm's
    // corpora now. What is left to test here is the adapter: egui's key and
    // pointer vocabulary onto the shared table's, and vt100's modes onto the
    // shared crate's.

    /// The keys [`xterm_key`] deliberately leaves unresolved even as a
    /// Ctrl/Alt chord, by [`egui::Key::name`]: no escape sequence of their
    /// own, and no single ASCII character for [`key_char`] to fold (Space,
    /// Minus and Quote are here because egui symbols them as non-ASCII).
    ///
    /// The list is exact, so an egui release that adds a key fails this test
    /// until someone decides which side of the line it falls on (the pattern
    /// of `keys.rs`, this kit's desktop key table).
    const UNREPRESENTED: &[&str] = &[
        "Copy",
        "Cut",
        "Paste",
        "Space",
        "Minus",
        "Quote",
        "F13",
        "F14",
        "F15",
        "F16",
        "F17",
        "F18",
        "F19",
        "F20",
        "F21",
        "F22",
        "F23",
        "F24",
        "F25",
        "F26",
        "F27",
        "F28",
        "F29",
        "F30",
        "F31",
        "F32",
        "F33",
        "F34",
        "F35",
        "BrowserBack",
        "ShiftLeft",
        "ShiftRight",
        "ControlLeft",
        "ControlRight",
        "AltLeft",
        "AltRight",
        "SuperLeft",
        "SuperRight",
        "IntlBackslash",
    ];

    /// Totality: every [`egui::Key`] either resolves through the shared table
    /// (by itself, or as a Ctrl/Alt chord) or is on the list above. The
    /// unrepresented keys must send nothing — not a plausible-looking wrong
    /// code.
    #[test]
    fn the_key_adapter_is_total_over_the_egui_key_enum() {
        let unresolved: Vec<&str> = Key::ALL
            .iter()
            .filter(|key| xterm_key(**key, Modifiers::CTRL).is_none())
            .map(|key| key.name())
            .collect();
        assert_eq!(unresolved, UNREPRESENTED);
        for &key in Key::ALL {
            if UNREPRESENTED.contains(&key.name()) {
                for m in [
                    egui::Modifiers::NONE,
                    egui::Modifiers::CTRL,
                    egui::Modifiers::ALT,
                ] {
                    assert_eq!(
                        encode_key(key, m, CursorKeys::Normal),
                        None,
                        "{} must send nothing",
                        key.name()
                    );
                }
            }
        }
    }

    /// Character keys reach the shared table as Ctrl/Alt chords only: a plain
    /// press arrives as `Event::Text`, which already carries the character.
    #[test]
    fn character_keys_encode_as_chords_only() {
        let none = egui::Modifiers::NONE;
        // Plain printable keys are Text events, never encoded here.
        assert_eq!(encode_key(Key::A, none, CursorKeys::Normal), None);
        // Ctrl+C → ETX; Alt+X → ESC-prefixed meta; Ctrl+digit has no control
        // byte, so it sends nothing.
        assert_eq!(
            encode_key(Key::C, egui::Modifiers::CTRL, CursorKeys::Normal),
            Some(vec![0x03])
        );
        assert_eq!(
            encode_key(Key::X, egui::Modifiers::ALT, CursorKeys::Normal),
            Some(vec![0x1b, b'x'])
        );
        assert_eq!(
            encode_key(Key::Num1, egui::Modifiers::CTRL, CursorKeys::Normal),
            None
        );
        // Named keys encode whatever modifier is held.
        assert_eq!(
            encode_key(Key::Enter, none, CursorKeys::Normal),
            Some(b"\r".to_vec())
        );
        assert_eq!(
            encode_key(Key::Delete, egui::Modifiers::CTRL, CursorKeys::Normal),
            Some(b"\x1b[3~".to_vec())
        );
    }

    /// F1 to F12 resolve through the shared table; F13 up send nothing.
    #[test]
    fn function_keys_reach_the_wire() {
        let none = egui::Modifiers::NONE;
        assert_eq!(
            encode_key(Key::F1, none, CursorKeys::Normal),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(
            encode_key(Key::F5, none, CursorKeys::Normal),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(encode_key(Key::F13, none, CursorKeys::Normal), None);
    }

    /// DECCKM is read from the vt100 screen, and the cursor keys switch to
    /// SS3 while it is set.
    #[test]
    fn application_cursor_mode_is_honoured() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        assert_eq!(cursor_keys(parser.screen()), CursorKeys::Normal);
        parser.process(b"\x1b[?1h");
        assert_eq!(cursor_keys(parser.screen()), CursorKeys::Application);
        parser.process(b"\x1b[?1l");
        assert_eq!(cursor_keys(parser.screen()), CursorKeys::Normal);

        let none = egui::Modifiers::NONE;
        assert_eq!(
            encode_key(Key::ArrowUp, none, CursorKeys::Normal),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_key(Key::ArrowUp, none, CursorKeys::Application),
            Some(b"\x1bOA".to_vec())
        );
    }

    /// The pointer adapter: egui's buttons onto the shared button codes, and
    /// vt100's tracking modes and encodings onto the shared crate's.
    #[test]
    fn the_mouse_adapter_maps_the_vocabularies() {
        assert_eq!(button_base(egui::PointerButton::Primary), Some(BUTTON_LEFT));
        assert_eq!(button_base(egui::PointerButton::Middle), Some(BUTTON_MIDDLE));
        assert_eq!(
            button_base(egui::PointerButton::Secondary),
            Some(BUTTON_RIGHT)
        );
        // The extra buttons have no xterm code; nothing is reported for them.
        assert_eq!(button_base(egui::PointerButton::Extra1), None);
        assert_eq!(button_base(egui::PointerButton::Extra2), None);

        assert_eq!(to_mouse_mode(vt100::MouseProtocolMode::None), MouseMode::None);
        assert_eq!(
            to_mouse_mode(vt100::MouseProtocolMode::Press),
            MouseMode::Press
        );
        assert_eq!(
            to_mouse_mode(vt100::MouseProtocolMode::PressRelease),
            MouseMode::PressRelease
        );
        assert_eq!(
            to_mouse_mode(vt100::MouseProtocolMode::ButtonMotion),
            MouseMode::ButtonMotion
        );
        assert_eq!(
            to_mouse_mode(vt100::MouseProtocolMode::AnyMotion),
            MouseMode::AnyMotion
        );
        assert_eq!(
            to_mouse_encoding(vt100::MouseProtocolEncoding::Default),
            MouseEncoding::Default
        );
        assert_eq!(
            to_mouse_encoding(vt100::MouseProtocolEncoding::Utf8),
            MouseEncoding::Utf8
        );
        assert_eq!(
            to_mouse_encoding(vt100::MouseProtocolEncoding::Sgr),
            MouseEncoding::Sgr
        );
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
