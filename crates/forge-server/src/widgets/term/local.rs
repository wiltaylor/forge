//! Local PTY sessions: spawn a shell as the server uid and bridge its
//! blocking IO to async channels.
//!
//! portable-pty is blocking, so each session runs three plain threads —
//! reader, writer, reaper — talking to the async session loop over bounded
//! tokio channels. Dropping [`PtyControl`] kills the child, which closes the
//! slave side and unblocks the reader thread, letting everything wind down.

use std::io::{Read, Write};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use tokio::sync::{mpsc, oneshot};

use super::super::CHANNEL_CAP;

const READ_BUF: usize = 8 * 1024;

/// Owns the pty master for resize and kills the child on drop.
pub(super) struct PtyControl {
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

impl PtyControl {
    pub(super) fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}

impl Drop for PtyControl {
    fn drop(&mut self) {
        let _ = self.killer.kill();
    }
}

/// Async ends of the bridge threads.
pub(super) struct PtyIo {
    /// Client → tty bytes, drained by the writer thread.
    pub(super) input: mpsc::Sender<Vec<u8>>,
    /// tty → client bytes, fed by the reader thread; closed on EOF.
    pub(super) output: mpsc::Receiver<Vec<u8>>,
    /// Child exit code, sent once by the reaper thread.
    pub(super) exit: oneshot::Receiver<i32>,
}

/// Open a pty, spawn `shell` in it and start the bridge threads.
pub(super) fn spawn(shell: &str, cols: u16, rows: u16) -> Result<(PtyControl, PtyIo), String> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("openpty failed: {e}"))?;

    let mut cmd = CommandBuilder::new(shell);
    cmd.env("TERM", "xterm-256color");
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("failed to spawn {shell}: {e}"))?;
    // Close our slave fd so the master reader sees EOF once the child exits.
    drop(pair.slave);

    let killer = child.clone_killer();
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("pty reader: {e}"))?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("pty writer: {e}"))?;

    let (out_tx, output) = mpsc::channel::<Vec<u8>>(CHANNEL_CAP);
    let (input, mut in_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAP);
    let (exit_tx, exit) = oneshot::channel();

    std::thread::spawn(move || {
        let mut buf = [0u8; READ_BUF];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // Err = session gone; stop pumping.
                    if out_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    std::thread::spawn(move || {
        while let Some(bytes) = in_rx.blocking_recv() {
            if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                break;
            }
        }
    });

    std::thread::spawn(move || {
        let code = child.wait().map(|s| s.exit_code() as i32).unwrap_or(-1);
        let _ = exit_tx.send(code);
    });

    Ok((
        PtyControl {
            master: pair.master,
            killer,
        },
        PtyIo {
            input,
            output,
            exit,
        },
    ))
}
