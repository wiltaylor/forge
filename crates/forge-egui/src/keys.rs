//! egui keys in the browser `KeyboardEvent` vocabulary this repo speaks.
//!
//! Two consumers read this table, and both want the same US-layout bridge:
//! the desktop viewer, which puts `code` + the produced character on the
//! remote-protocol wire (`docs/widgets-protocol.md` "Keymap is US-layout
//! v1"), and the block editor, which hands the same pair to
//! `forge_blocks::resolve_key`. One table, so the two cannot drift.
//!
//! [`egui::Key`] is logical — it names the character-producing key, not the
//! physical one — so this is a static approximation of the US layout. It is
//! total over the enum by test: every variant either names a code or is on
//! the list of keys deliberately left unnamed.

use egui::Key;

/// `KeyboardEvent.code` for a key. `None` = not representable (skip it).
///
/// Plain modifier keys (Shift/Control/Alt) return `None` on purpose: the
/// desktop widget synthesizes their transitions from [`egui::Modifiers`]
/// diffs. Super/Meta is the exception — `Modifiers` has no super field off
/// macOS, so the physical key events are forwarded directly.
pub(crate) fn code_str(key: Key) -> Option<&'static str> {
    use Key::*;
    Some(match key {
        // Commands / navigation.
        ArrowDown => "ArrowDown",
        ArrowLeft => "ArrowLeft",
        ArrowRight => "ArrowRight",
        ArrowUp => "ArrowUp",
        Escape => "Escape",
        Tab => "Tab",
        Backspace => "Backspace",
        Enter => "Enter",
        Space => "Space",
        Insert => "Insert",
        Delete => "Delete",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        // Punctuation: shifted logical keys collapse onto their physical key.
        Colon | Semicolon => "Semicolon",
        Comma => "Comma",
        Backslash | Pipe => "Backslash",
        Slash | Questionmark => "Slash",
        Exclamationmark => "Digit1",
        OpenBracket | OpenCurlyBracket => "BracketLeft",
        CloseBracket | CloseCurlyBracket => "BracketRight",
        Backtick => "Backquote",
        Minus => "Minus",
        Period => "Period",
        Plus | Equals => "Equal",
        Quote => "Quote",
        // Digits (egui does not distinguish the numpad).
        Num0 => "Digit0",
        Num1 => "Digit1",
        Num2 => "Digit2",
        Num3 => "Digit3",
        Num4 => "Digit4",
        Num5 => "Digit5",
        Num6 => "Digit6",
        Num7 => "Digit7",
        Num8 => "Digit8",
        Num9 => "Digit9",
        // Letters.
        A => "KeyA",
        B => "KeyB",
        C => "KeyC",
        D => "KeyD",
        E => "KeyE",
        F => "KeyF",
        G => "KeyG",
        H => "KeyH",
        I => "KeyI",
        J => "KeyJ",
        K => "KeyK",
        L => "KeyL",
        M => "KeyM",
        N => "KeyN",
        O => "KeyO",
        P => "KeyP",
        Q => "KeyQ",
        R => "KeyR",
        S => "KeyS",
        T => "KeyT",
        U => "KeyU",
        V => "KeyV",
        W => "KeyW",
        X => "KeyX",
        Y => "KeyY",
        Z => "KeyZ",
        // Function keys: F13+ resolve in neither forge-core keymap.
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        // Super/Meta: forwarded from physical key events (see module docs).
        SuperLeft => "MetaLeft",
        SuperRight => "MetaRight",
        // ISO 102nd key.
        IntlBackslash => "IntlBackslash",
        _ => return None,
    })
}

/// The character a key produces on a US layout (the protocol's `key` field).
/// `None` for non-printables — the VNC keysym path falls back to the code
/// table for those.
///
/// The desktop viewer's half of the bridge: the block editor reads the
/// character egui already resolved, off the text event that carries it.
#[cfg_attr(
    not(any(feature = "vnc", feature = "rdp")),
    allow(dead_code, reason = "the block editor reads only the code table")
)]
pub(crate) fn us_char(key: Key, shift: bool) -> Option<char> {
    use Key::*;
    let pair = |plain: char, shifted: char| Some(if shift { shifted } else { plain });
    match key {
        Space => Some(' '),
        // Punctuation rows. The already-shifted logical variants (Colon,
        // Pipe, …) ignore `shift`: egui resolved the character for us.
        Minus => pair('-', '_'),
        Equals => pair('=', '+'),
        Plus => Some('+'),
        OpenBracket => pair('[', '{'),
        CloseBracket => pair(']', '}'),
        OpenCurlyBracket => Some('{'),
        CloseCurlyBracket => Some('}'),
        Backslash | IntlBackslash => pair('\\', '|'),
        Pipe => Some('|'),
        Semicolon => pair(';', ':'),
        Colon => Some(':'),
        Quote => pair('\'', '"'),
        Backtick => pair('`', '~'),
        Comma => pair(',', '<'),
        Period => pair('.', '>'),
        Slash => pair('/', '?'),
        Questionmark => Some('?'),
        Exclamationmark => Some('!'),
        Num0 => pair('0', ')'),
        Num1 => pair('1', '!'),
        Num2 => pair('2', '@'),
        Num3 => pair('3', '#'),
        Num4 => pair('4', '$'),
        Num5 => pair('5', '%'),
        Num6 => pair('6', '^'),
        Num7 => pair('7', '&'),
        Num8 => pair('8', '*'),
        Num9 => pair('9', '('),
        A => pair('a', 'A'),
        B => pair('b', 'B'),
        C => pair('c', 'C'),
        D => pair('d', 'D'),
        E => pair('e', 'E'),
        F => pair('f', 'F'),
        G => pair('g', 'G'),
        H => pair('h', 'H'),
        I => pair('i', 'I'),
        J => pair('j', 'J'),
        K => pair('k', 'K'),
        L => pair('l', 'L'),
        M => pair('m', 'M'),
        N => pair('n', 'N'),
        O => pair('o', 'O'),
        P => pair('p', 'P'),
        Q => pair('q', 'Q'),
        R => pair('r', 'R'),
        S => pair('s', 'S'),
        T => pair('t', 'T'),
        U => pair('u', 'U'),
        V => pair('v', 'V'),
        W => pair('w', 'W'),
        X => pair('x', 'X'),
        Y => pair('y', 'Y'),
        Z => pair('z', 'Z'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys [`code_str`] deliberately leaves unnamed, by
    /// [`egui::Key::name`]. F13+ and the media keys resolve in neither
    /// forge-core keymap, and the plain modifiers are synthesized from
    /// [`egui::Modifiers`] diffs rather than mapped.
    ///
    /// The list is exact, so an egui release that adds a key fails this test
    /// until someone decides which side of the line it falls on.
    const UNNAMED: &[&str] = &[
        "Copy",
        "Cut",
        "Paste",
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
    ];

    /// Totality: every [`egui::Key`] either names a code or is on the list
    /// above. Nothing falls through the table by accident.
    #[test]
    fn the_code_table_is_total_over_the_egui_key_enum() {
        let unnamed: Vec<&str> = Key::ALL
            .iter()
            .filter(|key| code_str(**key).is_none())
            .map(|key| key.name())
            .collect();
        assert_eq!(unnamed, UNNAMED);
    }

    /// A named key that produces a character is a printable; a named key
    /// that produces none is a command key. Nothing in between.
    #[test]
    fn every_printable_is_named() {
        for &key in Key::ALL {
            if us_char(key, false).is_some() {
                assert!(
                    code_str(key).is_some(),
                    "{} types but has no code",
                    key.name()
                );
            }
        }
    }

    #[test]
    fn us_shift_pairs() {
        assert_eq!(us_char(Key::A, false), Some('a'));
        assert_eq!(us_char(Key::A, true), Some('A'));
        assert_eq!(us_char(Key::Num1, false), Some('1'));
        assert_eq!(us_char(Key::Num1, true), Some('!'));
        assert_eq!(us_char(Key::Semicolon, false), Some(';'));
        assert_eq!(us_char(Key::Semicolon, true), Some(':'));
        assert_eq!(us_char(Key::Backtick, true), Some('~'));
        assert_eq!(us_char(Key::Space, true), Some(' '));
        // Non-printables carry no `key` field.
        assert_eq!(us_char(Key::Enter, false), None);
        assert_eq!(us_char(Key::ArrowUp, false), None);
    }

    /// The layout this table approximates is the one `forge_blocks` writes
    /// its corpus keys against. Where both name the key that types a
    /// character, they must name the same one.
    ///
    /// `IntlBackslash` is the exception, and the reason the check is written
    /// this way round: the ISO 102nd key types `\` on a US layout but is a
    /// different physical key from `Backslash`, which is the code
    /// `forge_blocks` gives that character.
    #[cfg(feature = "blocks")]
    #[test]
    fn the_us_table_agrees_with_the_forge_blocks_layout() {
        for &key in Key::ALL {
            if key == Key::IntlBackslash {
                continue;
            }
            for shift in [false, true] {
                let Some(c) = us_char(key, shift) else {
                    continue;
                };
                let shared = forge_blocks::Key::typed(c);
                assert_eq!(
                    code_str(key),
                    Some(shared.code.as_str()),
                    "{} types {c:?}, which forge_blocks codes as {:?}",
                    key.name(),
                    shared.code,
                );
            }
        }
    }
}
