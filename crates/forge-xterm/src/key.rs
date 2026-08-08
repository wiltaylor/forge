//! Key presses: the bytes each key sends to the program in the terminal.
//!
//! One table, called by both kits. Each kit maps its own key type (crossterm's
//! `KeyCode`, egui's `Key`) onto [`Key`] and asks [`encode`] for the bytes.
//! [`encode`] answers `None` when the key has no representation, so a kit
//! sends nothing rather than a plausible-looking wrong code.
//!
//! Two things the table does not decide, because they belong to the kit:
//!
//! - **Which keys to ask about.** A kit that receives typed text as its own
//!   event must not also ask here for a plain [`Key::Char`], or the character
//!   goes out twice. A kit that keeps [`Key::Tab`] for focus traversal simply
//!   does not ask about it.
//! - **What the character is.** Shift changes which character the key
//!   produces, not the bytes that character sends, so the kit resolves the
//!   layout and passes the result in [`Key::Char`].

use crate::{push_utf8, Modifiers};

/// A key press, in the vocabulary both kits map onto.
///
/// [`Key::Char`] carries the character the key produces after the keyboard
/// layout is applied. The rest are the keys that send an escape sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    /// A character key: the character it produces, layout applied.
    Char(char),
    /// Return.
    Enter,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Escape.
    Escape,
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor right.
    Right,
    /// Cursor left.
    Left,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete (forward delete).
    Delete,
    /// A function key, numbered from 1. F1 to F12 have sequences here;
    /// anything else resolves to nothing.
    Function(u8),
}

/// What the running program asked the cursor keys to send.
///
/// DECCKM (`?1h`) puts them in application mode, which full-screen programs
/// set so that the arrow keys are distinct from the CSI sequences a program
/// may print. `?1l` puts them back. The cursor keys are the four arrows plus
/// Home and End — terminfo's `khome` is `\EOH` under `smkx`, so a program that
/// asked for application mode expects the SS3 form of those six.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CursorKeys {
    /// The cursor keys send CSI: `ESC [ A`, `ESC [ H`.
    #[default]
    Normal,
    /// The cursor keys send SS3: `ESC O A`, `ESC O H`.
    Application,
}

/// The bytes `key` sends, or `None` when it has no representation.
///
/// The modifiers reach the wire on character keys only: Ctrl folds a letter to
/// its control byte, and Alt prefixes ESC (readline's meta). The keys that send
/// an escape sequence send the same one whatever modifier is held — this table
/// does not produce xterm's modified forms (`ESC [ 1;5 A` and friends).
pub fn encode(key: Key, modifiers: Modifiers, cursor: CursorKeys) -> Option<Vec<u8>> {
    // The cursor keys: CSI while the mode is normal, SS3 while it is
    // application. Everything else sends one sequence in both modes.
    let cursor_key = |ch: u8| -> Vec<u8> {
        match cursor {
            CursorKeys::Normal => vec![0x1b, b'[', ch],
            CursorKeys::Application => vec![0x1b, b'O', ch],
        }
    };
    Some(match key {
        Key::Char(c) => return char_bytes(c, modifiers),
        Key::Enter => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab => vec![b'\t'],
        Key::Escape => vec![0x1b],
        Key::Up => cursor_key(b'A'),
        Key::Down => cursor_key(b'B'),
        Key::Right => cursor_key(b'C'),
        Key::Left => cursor_key(b'D'),
        Key::Home => cursor_key(b'H'),
        Key::End => cursor_key(b'F'),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
        Key::Insert => b"\x1b[2~".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        // F1 to F4 are SS3, as they are on the VT220 keypad; F5 up are tilde
        // sequences, and xterm skips 16 and 22 in their numbering.
        Key::Function(1) => b"\x1bOP".to_vec(),
        Key::Function(2) => b"\x1bOQ".to_vec(),
        Key::Function(3) => b"\x1bOR".to_vec(),
        Key::Function(4) => b"\x1bOS".to_vec(),
        Key::Function(5) => b"\x1b[15~".to_vec(),
        Key::Function(6) => b"\x1b[17~".to_vec(),
        Key::Function(7) => b"\x1b[18~".to_vec(),
        Key::Function(8) => b"\x1b[19~".to_vec(),
        Key::Function(9) => b"\x1b[20~".to_vec(),
        Key::Function(10) => b"\x1b[21~".to_vec(),
        Key::Function(11) => b"\x1b[23~".to_vec(),
        Key::Function(12) => b"\x1b[24~".to_vec(),
        // There is no F0, and F13 up are not in this table.
        Key::Function(_) => return None,
    })
}

/// A character key: itself, its control byte, or ESC and either of those.
///
/// The two modifiers compose, as they do in xterm: Alt is a prefix, Ctrl folds
/// the character, and Ctrl+Alt+C is ESC then `0x03`.
fn char_bytes(c: char, modifiers: Modifiers) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    // Alt prefixes ESC, which is how readline reads a meta chord.
    if modifiers.alt {
        out.push(0x1b);
    }
    if modifiers.ctrl {
        // Ctrl folds a letter to the low five bits of its upper-case form:
        // Ctrl+C is 0x03. Only the letters have one — Ctrl and a digit sends
        // nothing rather than a byte the program did not ask for.
        let upper = c.to_ascii_uppercase();
        if !upper.is_ascii_alphabetic() {
            return None;
        }
        out.push((upper as u8) & 0x1f);
    } else {
        push_utf8(&mut out, c as u32);
    }
    Some(out)
}
