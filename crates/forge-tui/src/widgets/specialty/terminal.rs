//! Embedded terminal (cargo feature `term`): a local PTY (portable-pty)
//! parsed by vt100 and painted into the buffer. Drain PTY output on the
//! runtime tick; route keys with `handle_key` while the pane is focused
//! (everything is forwarded — pick a focus-escape chord at the app level,
//! Tab is NOT forwarded so the default focus traversal still works).

use crate::event::{is_press, Outcome};
use crate::theme::{default_theme, Theme};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, PtySize};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(std::io::Error::other)?;
        let mut child = pty.slave.spawn_command(cmd).map_err(std::io::Error::other)?;
        let killer = child.clone_killer();
        // Reap the child so it never zombies; the reader below observes EOF.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        let mut reader = pty.master.try_clone_reader().map_err(std::io::Error::other)?;
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
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        let _ = self.killer.kill();
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
        let t = self.theme.unwrap_or_else(|| default_theme());
        state.resize(area.height, area.width);
        let screen = state.parser.screen();
        buf.set_style(area, Style::new().bg(t.bg[0]));
        for row in 0..area.height.min(state.size.0) {
            for col in 0..area.width.min(state.size.1) {
                let Some(cell) = screen.cell(row, col) else { continue };
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
