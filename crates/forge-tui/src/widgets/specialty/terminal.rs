//! Embedded terminal (cargo feature `term`): a local PTY (portable-pty)
//! parsed by vt100 and painted into the buffer. Drain PTY output on the
//! runtime tick; route keys with `handle_key` while the pane is focused
//! (everything is forwarded — pick a focus-escape chord at the app level,
//! Tab is NOT forwarded so the default focus traversal still works).

use crate::event::{in_area, is_press, Outcome};
use crate::theme::{default_theme, Theme};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::StatefulWidget;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, TryRecvError};

pub struct TerminalState {
    parser: vt100::Parser,
    writer: Box<dyn Write + Send>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send>,
    rx: Receiver<Vec<u8>>,
    exited: bool,
    size: (u16, u16),
    /// The pane rect from the last `render`, used to map absolute crossterm
    /// mouse coords to cells in `handle_mouse`.
    last_area: Rect,
    /// Cell of the last reported mouse motion, so button-motion/any-motion
    /// modes only report when the pointer crosses into a new cell.
    last_mouse_cell: Option<(u16, u16)>,
}

impl std::fmt::Debug for TerminalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalState")
            .field("exited", &self.exited)
            .field("size", &self.size)
            .finish()
    }
}

impl TerminalState {
    /// Spawn `cmd` on a fresh PTY of `rows`×`cols`.
    pub fn spawn(cmd: CommandBuilder, rows: u16, cols: u16) -> std::io::Result<TerminalState> {
        let pty = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(std::io::Error::other)?;
        let mut child = pty
            .slave
            .spawn_command(cmd)
            .map_err(std::io::Error::other)?;
        let killer = child.clone_killer();
        // Reap the child so it never zombies; the reader below observes EOF.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        let mut reader = pty
            .master
            .try_clone_reader()
            .map_err(std::io::Error::other)?;
        let writer = pty.master.take_writer().map_err(std::io::Error::other)?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Ok(TerminalState {
            parser: vt100::Parser::new(rows, cols, 2000),
            writer,
            _master: pty.master,
            killer,
            rx,
            exited: false,
            size: (rows, cols),
            last_area: Rect::default(),
            last_mouse_cell: None,
        })
    }

    /// Shell convenience: `$SHELL` (or sh/cmd).
    pub fn spawn_shell(rows: u16, cols: u16) -> std::io::Result<TerminalState> {
        #[cfg(unix)]
        let program = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        #[cfg(windows)]
        let program = "cmd.exe".to_string();
        let mut cmd = CommandBuilder::new(program);
        cmd.env("TERM", "xterm-256color");
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }
        TerminalState::spawn(cmd, rows, cols)
    }

    /// Pump pending PTY output into the vt100 screen. Call on the runtime
    /// tick; returns true when new output arrived (repaint).
    pub fn drain(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.rx.try_recv() {
                Ok(chunk) => {
                    self.parser.process(&chunk);
                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.exited = true;
                    break;
                }
            }
        }
        changed
    }

    pub fn exited(&self) -> bool {
        self.exited
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if (rows, cols) != self.size && rows > 0 && cols > 0 {
            self.size = (rows, cols);
            self.parser.set_size(rows, cols);
            let _ = self._master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn write_str(&mut self, s: &str) {
        let _ = self.writer.write_all(s.as_bytes());
        let _ = self.writer.flush();
    }

    /// Forward a key to the PTY. Tab/BackTab are left to the app (focus
    /// traversal); everything else is encoded xterm-style.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) || self.exited {
            return Outcome::Ignored;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let bytes: Vec<u8> = match key.code {
            KeyCode::Tab | KeyCode::BackTab => return Outcome::Ignored,
            KeyCode::Char(c) if ctrl => {
                let u = c.to_ascii_uppercase();
                if u.is_ascii_uppercase() {
                    vec![(u as u8) & 0x1f]
                } else {
                    return Outcome::Ignored;
                }
            }
            KeyCode::Char(c) => {
                let mut b = [0u8; 4];
                c.encode_utf8(&mut b).as_bytes().to_vec()
            }
            KeyCode::Enter => b"\r".to_vec(),
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => b"\x1b[A".to_vec(),
            KeyCode::Down => b"\x1b[B".to_vec(),
            KeyCode::Right => b"\x1b[C".to_vec(),
            KeyCode::Left => b"\x1b[D".to_vec(),
            KeyCode::Home => b"\x1b[H".to_vec(),
            KeyCode::End => b"\x1b[F".to_vec(),
            KeyCode::PageUp => b"\x1b[5~".to_vec(),
            KeyCode::PageDown => b"\x1b[6~".to_vec(),
            KeyCode::Delete => b"\x1b[3~".to_vec(),
            KeyCode::Insert => b"\x1b[2~".to_vec(),
            _ => return Outcome::Ignored,
        };
        let _ = self.writer.write_all(&bytes);
        let _ = self.writer.flush();
        Outcome::Consumed
    }

    /// Forward a mouse event to the PTY as an xterm mouse report, but only when
    /// the running program has enabled mouse tracking (DECSET `?1000`/`?1002`/
    /// `?1003`, …). A plain shell reports `MouseProtocolMode::None`, so clicks
    /// and scroll are ignored here and left for the app to handle (focus, etc).
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> Outcome {
        if self.exited {
            return Outcome::Ignored;
        }
        let (mode, encoding) = {
            let screen = self.parser.screen();
            (
                screen.mouse_protocol_mode(),
                screen.mouse_protocol_encoding(),
            )
        };
        if mode == vt100::MouseProtocolMode::None || !in_area(&ev, self.last_area) {
            return Outcome::Ignored;
        }
        let (button, motion, release) = match ev.kind {
            MouseEventKind::Down(b) => (button_code(b), false, false),
            MouseEventKind::Up(b) => (button_code(b), false, true),
            MouseEventKind::Drag(b) => (button_code(b), true, false),
            MouseEventKind::Moved => (BUTTON_NONE, true, false),
            MouseEventKind::ScrollUp => (WHEEL_UP, false, false),
            MouseEventKind::ScrollDown => (WHEEL_DOWN, false, false),
            MouseEventKind::ScrollLeft => (WHEEL_LEFT, false, false),
            MouseEventKind::ScrollRight => (WHEEL_RIGHT, false, false),
        };
        if !mode_allows(mode, button, motion, release) {
            return Outcome::Ignored;
        }
        let col = ev.column.saturating_sub(self.last_area.x);
        let row = ev.row.saturating_sub(self.last_area.y);
        // Button-motion / any-motion modes fire once per cell crossing.
        if motion && self.last_mouse_cell == Some((col, row)) {
            return Outcome::Ignored;
        }
        self.last_mouse_cell = Some((col, row));
        let m = ev.modifiers;
        let bytes = encode_mouse(
            encoding,
            button,
            motion,
            release,
            col,
            row,
            m.contains(KeyModifiers::SHIFT),
            m.contains(KeyModifiers::ALT),
            m.contains(KeyModifiers::CONTROL),
        );
        let _ = self.writer.write_all(&bytes);
        let _ = self.writer.flush();
        Outcome::Consumed
    }
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        let _ = self.killer.kill();
    }
}

// xterm mouse button codes (the low bits of `cb`, before modifier/motion bits).
const BUTTON_NONE: u16 = 3; // no button held (bare motion, X10 release)
const WHEEL_UP: u16 = 64;
const WHEEL_DOWN: u16 = 65;
const WHEEL_LEFT: u16 = 66;
const WHEEL_RIGHT: u16 = 67;

fn button_code(b: MouseButton) -> u16 {
    match b {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
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

fn map_color(c: vt100::Color, default: Color) -> Color {
    match c {
        vt100::Color::Default => default,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// The terminal pane view. Resizes the PTY to the render area.
#[derive(Clone, Debug, Default)]
pub struct Terminal<'a> {
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Terminal<'a> {
    pub fn new() -> Terminal<'a> {
        Terminal::default()
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Terminal<'a> {
    type State = TerminalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TerminalState) {
        if area.is_empty() {
            return;
        }
        state.last_area = area;
        let t = self.theme.unwrap_or_else(|| default_theme());
        state.resize(area.height, area.width);
        let screen = state.parser.screen();
        buf.set_style(area, Style::new().bg(t.bg[0]));
        for row in 0..area.height.min(state.size.0) {
            for col in 0..area.width.min(state.size.1) {
                let Some(cell) = screen.cell(row, col) else {
                    continue;
                };
                let x = area.x + col;
                let y = area.y + row;
                let mut style = Style::new()
                    .fg(map_color(cell.fgcolor(), t.fg[0]))
                    .bg(map_color(cell.bgcolor(), t.bg[0]));
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if cell.inverse() {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                let contents = cell.contents();
                let symbol = if contents.is_empty() { " " } else { &contents };
                buf.set_string(x, y, symbol, style);
            }
        }
        if self.focused && !screen.hide_cursor() {
            let (cr, cc) = screen.cursor_position();
            if cr < area.height && cc < area.width {
                buf.set_style(
                    Rect::new(area.x + cc, area.y + cr, 1, 1),
                    Style::new().add_modifier(Modifier::REVERSED),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vt100::{MouseProtocolEncoding as Enc, MouseProtocolMode as Mode};

    fn sgr(button: u16, motion: bool, release: bool, col: u16, row: u16) -> String {
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
    }

    #[test]
    fn sgr_press_release_and_coords() {
        // Left press at cell (0,0) → 1-based coords, final 'M'.
        assert_eq!(sgr(0, false, false, 0, 0), "\x1b[<0;1;1M");
        // Left release → same cb, final 'm'.
        assert_eq!(sgr(0, false, true, 4, 2), "\x1b[<0;5;3m");
        // Right button drag sets the motion bit (+32) and keeps button id.
        assert_eq!(sgr(2, true, false, 9, 1), "\x1b[<34;10;2M");
    }

    #[test]
    fn sgr_modifiers_and_wheel() {
        // Ctrl(+16)+Shift(+4) left press = cb 20.
        let bytes = encode_mouse(Enc::Sgr, 0, false, false, 0, 0, true, false, true);
        assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[<20;1;1M");
        // Wheel up is a press with cb 64.
        assert_eq!(sgr(WHEEL_UP, false, false, 3, 3), "\x1b[<64;4;4M");
    }

    #[test]
    fn x10_default_bytes() {
        // Left press at (0,0): ESC [ M then 32+cb, 32+col+1, 32+row+1.
        let b = encode_mouse(Enc::Default, 0, false, false, 0, 0, false, false, false);
        assert_eq!(b, vec![0x1b, b'[', b'M', 32, 33, 33]);
        // Release drops the button id to "none" (3), regardless of press button.
        let b = encode_mouse(Enc::Default, 2, false, true, 0, 0, false, false, false);
        assert_eq!(b, vec![0x1b, b'[', b'M', 32 + 3, 33, 33]);
    }

    #[test]
    fn mode_gating() {
        // Press-only mode: down + wheel yes; up/drag/move no.
        assert!(mode_allows(Mode::Press, 0, false, false)); // down
        assert!(mode_allows(Mode::Press, WHEEL_UP, false, false)); // wheel
        assert!(!mode_allows(Mode::Press, 0, false, true)); // up
        assert!(!mode_allows(Mode::Press, 0, true, false)); // drag
                                                            // PressRelease: up yes, bare motion no.
        assert!(mode_allows(Mode::PressRelease, 0, false, true));
        assert!(!mode_allows(Mode::PressRelease, 0, true, false));
        // ButtonMotion: drag (button held) yes, bare move no.
        assert!(mode_allows(Mode::ButtonMotion, 0, true, false));
        assert!(!mode_allows(Mode::ButtonMotion, BUTTON_NONE, true, false));
        // AnyMotion: bare move yes.
        assert!(mode_allows(Mode::AnyMotion, BUTTON_NONE, true, false));
        // None: nothing.
        assert!(!mode_allows(Mode::None, 0, false, false));
    }

    fn left_down(col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    /// End-to-end over a live PTY: a plain shell reports no mouse tracking, so
    /// clicks are ignored (left for the app); once a program enables tracking
    /// the same click is forwarded (`Consumed`).
    #[test]
    fn handle_mouse_gated_on_tracking_mode() {
        use std::time::{Duration, Instant};

        // A shell that turns on SGR mouse tracking, then idles so the session
        // stays live while we click.
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.arg("-c");
        cmd.arg("printf '\\033[?1000h\\033[?1006h'; sleep 5");
        let mut term = TerminalState::spawn(cmd, 24, 80).unwrap();
        term.last_area = Rect::new(0, 0, 80, 24);

        // Before the program's DECSET is processed, tracking is off → ignored.
        assert_eq!(term.parser.screen().mouse_protocol_mode(), Mode::None);
        assert_eq!(term.handle_mouse(left_down(3, 2)), Outcome::Ignored);

        // Pump PTY output until the DECSET flips the parser into tracking mode.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            term.drain();
            if term.parser.screen().mouse_protocol_mode() != Mode::None {
                break;
            }
            assert!(Instant::now() < deadline, "mouse tracking never enabled");
            std::thread::sleep(Duration::from_millis(20));
        }

        // Now the click is forwarded, and one outside the pane still isn't.
        assert_eq!(term.handle_mouse(left_down(3, 2)), Outcome::Consumed);
        assert_eq!(term.handle_mouse(left_down(200, 200)), Outcome::Ignored);
    }
}
