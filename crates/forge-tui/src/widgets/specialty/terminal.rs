//! Embedded terminal (cargo feature `term`): a local PTY (portable-pty)
//! parsed by vt100 and painted into the buffer. Drain PTY output on the
//! runtime tick; route keys with `handle_key` while the pane is focused
//! (everything is forwarded — pick a focus-escape chord at the app level,
//! Tab is NOT forwarded so the default focus traversal still works).

use crate::event::{in_area, is_press, Outcome};
use crate::theme::{Surface, TextRole};
use crate::widgets::paint;
use forge_xterm::key::{self, CursorKeys, Key};
use forge_xterm::mouse::{
    self, MouseEncoding, MouseMode, MouseReport, BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_NONE,
    BUTTON_RIGHT, WHEEL_DOWN, WHEEL_LEFT, WHEEL_RIGHT, WHEEL_UP,
};
use forge_xterm::Modifiers;
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
            // vt100 0.16 dropped Parser::set_size (it only forwarded here).
            self.parser.screen_mut().set_size(rows, cols);
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
    /// traversal); everything else goes through the shared xterm table,
    /// honouring the cursor-key mode (DECCKM) the running program set.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) || self.exited {
            return Outcome::Ignored;
        }
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            return Outcome::Ignored;
        }
        let cursor = cursor_keys(self.parser.screen());
        let Some(bytes) = key_bytes(key.code, to_modifiers(key.modifiers), cursor) else {
            return Outcome::Ignored;
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
                mouse_mode(screen.mouse_protocol_mode()),
                mouse_encoding(screen.mouse_protocol_encoding()),
            )
        };
        if mode == MouseMode::None || !in_area(&ev, self.last_area) {
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
        let report = MouseReport {
            button,
            motion,
            release,
            col: ev.column.saturating_sub(self.last_area.x),
            row: ev.row.saturating_sub(self.last_area.y),
            modifiers: to_modifiers(ev.modifiers),
        };
        if !mouse::is_reported(&report, mode) {
            return Outcome::Ignored;
        }
        // Button-motion / any-motion modes fire once per cell crossing.
        if motion && self.last_mouse_cell == Some((report.col, report.row)) {
            return Outcome::Ignored;
        }
        self.last_mouse_cell = Some((report.col, report.row));
        let bytes = mouse::encode(&report, encoding);
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

/// crossterm's key vocabulary mapped onto the shared table's ([`Key`]).
/// `None` = the key has no bytes to send, so the kit sends nothing. The match
/// is exhaustive on purpose: a new crossterm variant fails compilation here
/// rather than silently sending nothing.
fn xterm_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Enter => Key::Enter,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Tab => Key::Tab,
        KeyCode::Esc => Key::Escape,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Right => Key::Right,
        KeyCode::Left => Key::Left,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Insert => Key::Insert,
        KeyCode::Delete => Key::Delete,
        KeyCode::F(n) => Key::Function(n),
        // BackTab never reaches the table — `handle_key` keeps both tab forms
        // for focus traversal — and the rest have no xterm bytes to send.
        KeyCode::BackTab
        | KeyCode::Null
        | KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => return None,
    })
}

/// The bytes a crossterm key sends, through the shared table.
fn key_bytes(code: KeyCode, modifiers: Modifiers, cursor: CursorKeys) -> Option<Vec<u8>> {
    key::encode(xterm_key(code)?, modifiers, cursor)
}

/// The cursor-key mode the running program asked for (DECCKM `?1h`/`?1l`).
fn cursor_keys(screen: &vt100::Screen) -> CursorKeys {
    if screen.application_cursor() {
        CursorKeys::Application
    } else {
        CursorKeys::Normal
    }
}

/// crossterm's modifier set in the shared crate's vocabulary.
fn to_modifiers(m: KeyModifiers) -> Modifiers {
    Modifiers {
        shift: m.contains(KeyModifiers::SHIFT),
        alt: m.contains(KeyModifiers::ALT),
        ctrl: m.contains(KeyModifiers::CONTROL),
    }
}

fn button_code(b: MouseButton) -> u16 {
    match b {
        MouseButton::Left => BUTTON_LEFT,
        MouseButton::Middle => BUTTON_MIDDLE,
        MouseButton::Right => BUTTON_RIGHT,
    }
}

/// vt100's tracking-mode vocabulary in the shared crate's.
fn mouse_mode(mode: vt100::MouseProtocolMode) -> MouseMode {
    match mode {
        vt100::MouseProtocolMode::None => MouseMode::None,
        vt100::MouseProtocolMode::Press => MouseMode::Press,
        vt100::MouseProtocolMode::PressRelease => MouseMode::PressRelease,
        vt100::MouseProtocolMode::ButtonMotion => MouseMode::ButtonMotion,
        vt100::MouseProtocolMode::AnyMotion => MouseMode::AnyMotion,
    }
}

/// vt100's encoding vocabulary in the shared crate's.
fn mouse_encoding(encoding: vt100::MouseProtocolEncoding) -> MouseEncoding {
    match encoding {
        vt100::MouseProtocolEncoding::Default => MouseEncoding::Default,
        vt100::MouseProtocolEncoding::Utf8 => MouseEncoding::Utf8,
        vt100::MouseProtocolEncoding::Sgr => MouseEncoding::Sgr,
    }
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
pub struct Terminal {
    focused: bool,
}

impl Terminal {
    pub fn new() -> Terminal {
        Terminal::default()
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl StatefulWidget for Terminal {
    type State = TerminalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TerminalState) {
        paint(area, |t| {
            state.last_area = area;
            state.resize(area.height, area.width);
            let screen = state.parser.screen();
            buf.set_style(area, Style::new().bg(t.surface(Surface::Page)));
            for row in 0..area.height.min(state.size.0) {
                for col in 0..area.width.min(state.size.1) {
                    let Some(cell) = screen.cell(row, col) else {
                        continue;
                    };
                    let x = area.x + col;
                    let y = area.y + row;
                    let mut style = Style::new()
                        .fg(map_color(cell.fgcolor(), t.text(TextRole::Primary)))
                        .bg(map_color(cell.bgcolor(), t.surface(Surface::Page)));
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
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{MediaKeyCode, ModifierKeyCode};

    // The encoding tests (bytes per event, mode gating) live in forge-xterm's
    // corpora now. What is left to test here is the adapter: crossterm's key
    // vocabulary onto the shared table, and vt100's modes onto the shared
    // crate's.

    /// Every crossterm `KeyCode` variant, with a sample payload where one is
    /// needed. The exhaustive match in [`xterm_key`] is the compile-time
    /// guard; this list is the runtime half, so keep the two in step.
    const ALL_KEY_CODES: &[KeyCode] = &[
        KeyCode::Backspace,
        KeyCode::Enter,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Delete,
        KeyCode::Insert,
        KeyCode::F(1),
        KeyCode::Char('a'),
        KeyCode::Null,
        KeyCode::Esc,
        KeyCode::CapsLock,
        KeyCode::ScrollLock,
        KeyCode::NumLock,
        KeyCode::PrintScreen,
        KeyCode::Pause,
        KeyCode::Menu,
        KeyCode::KeypadBegin,
        KeyCode::Media(MediaKeyCode::Play),
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
    ];

    /// The variants with no xterm bytes. They must produce nothing — not a
    /// plausible-looking wrong code.
    const UNREPRESENTED: &[KeyCode] = &[
        KeyCode::BackTab,
        KeyCode::Null,
        KeyCode::CapsLock,
        KeyCode::ScrollLock,
        KeyCode::NumLock,
        KeyCode::PrintScreen,
        KeyCode::Pause,
        KeyCode::Menu,
        KeyCode::KeypadBegin,
        KeyCode::Media(MediaKeyCode::Play),
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
    ];

    /// Totality: every crossterm variant either resolves through the shared
    /// table or is on the unrepresented list. Nothing falls through by
    /// accident.
    ///
    /// Both halves reach the wire: a represented key must produce bytes, and
    /// an unrepresented key must produce nothing — not a plausible-looking
    /// wrong code.
    #[test]
    fn the_key_adapter_is_total_over_the_crossterm_enum() {
        let unresolved: Vec<KeyCode> = ALL_KEY_CODES
            .iter()
            .copied()
            .filter(|code| xterm_key(*code).is_none())
            .collect();
        assert_eq!(unresolved, UNREPRESENTED);
        for code in ALL_KEY_CODES {
            if UNREPRESENTED.contains(code) {
                assert_eq!(
                    key_bytes(*code, Modifiers::NONE, CursorKeys::Normal),
                    None,
                    "{code:?} must send nothing"
                );
            } else {
                assert!(
                    key_bytes(*code, Modifiers::NONE, CursorKeys::Normal).is_some(),
                    "{code:?} must reach the wire"
                );
            }
        }
    }

    /// The divergence this ticket closes: the old local table had no function
    /// keys. F1 to F12 resolve; F0 and F13 up send nothing.
    #[test]
    fn function_keys_reach_the_wire() {
        assert_eq!(
            key_bytes(KeyCode::F(1), Modifiers::NONE, CursorKeys::Normal),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(
            key_bytes(KeyCode::F(5), Modifiers::NONE, CursorKeys::Normal),
            Some(b"\x1b[15~".to_vec())
        );
        for n in 1..=12u8 {
            assert!(
                key_bytes(KeyCode::F(n), Modifiers::NONE, CursorKeys::Normal).is_some(),
                "F{n} must resolve"
            );
        }
        for n in [0u8, 13, 255] {
            assert_eq!(
                key_bytes(KeyCode::F(n), Modifiers::NONE, CursorKeys::Normal),
                None,
                "F{n} must send nothing"
            );
        }
    }

    /// The other half of the divergence: DECCKM is read from the screen, and
    /// the arrows switch to SS3 while it is set.
    #[test]
    fn application_cursor_mode_is_honoured() {
        let mut parser = vt100::Parser::new(24, 80, 0);
        assert_eq!(cursor_keys(parser.screen()), CursorKeys::Normal);
        parser.process(b"\x1b[?1h");
        assert_eq!(cursor_keys(parser.screen()), CursorKeys::Application);
        parser.process(b"\x1b[?1l");
        assert_eq!(cursor_keys(parser.screen()), CursorKeys::Normal);

        assert_eq!(
            key_bytes(KeyCode::Up, Modifiers::NONE, CursorKeys::Normal),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_bytes(KeyCode::Up, Modifiers::NONE, CursorKeys::Application),
            Some(b"\x1bOA".to_vec())
        );
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
        cmd.arg("printf '\\033[?1000h\\033[?1006h'; sleep 10");
        let mut term = TerminalState::spawn(cmd, 24, 80).unwrap();

        // Render once, the way an app does, so the pane knows its rectangle.
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        Terminal::new().render(area, &mut buf, &mut term);

        // Nothing is drained yet, so the program's DECSET has not been
        // processed: tracking is off → ignored.
        assert_eq!(term.handle_mouse(left_down(3, 2)), Outcome::Ignored);

        // Pump PTY output until the DECSET takes effect and the same click is
        // forwarded.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            term.drain();
            if term.handle_mouse(left_down(3, 2)) == Outcome::Consumed {
                break;
            }
            assert!(Instant::now() < deadline, "mouse tracking never enabled");
            std::thread::sleep(Duration::from_millis(20));
        }

        // Tracking is on, but a click outside the pane still isn't forwarded.
        assert_eq!(term.handle_mouse(left_down(200, 200)), Outcome::Ignored);
    }
}
