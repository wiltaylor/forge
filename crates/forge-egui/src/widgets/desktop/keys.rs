//! egui key → browser-style key event fields for the desktop wire protocol.
//!
//! The protocol ([`forge_core::widgets::proto::DesktopClientMsg::Key`]) wants
//! `KeyboardEvent.code` (layout-independent physical key) plus the produced
//! character in `key`. egui gives us [`egui::Key`] — logical, but with
//! `physical_key` preferred by the caller — so this module is the static
//! US-layout bridge, exactly the v1 approximation the web widget ships
//! (docs/widgets-protocol.md "Keymap is US-layout v1").
//!
//! Every string returned by [`code_str`] must resolve through **both**
//! forge-core keymaps (`keysym` for VNC, `scancode` for RDP) — enforced by
//! the tests below. That is why F13+ and media keys map to `None`: they'd be
//! dead codes on the wire.

use egui::Key;

/// `KeyboardEvent.code` for a key. `None` = not representable (skip it).
///
/// Plain modifier keys (Shift/Control/Alt) return `None` on purpose: the
/// widget synthesizes their transitions from [`egui::Modifiers`] diffs.
/// Super/Meta is the exception — `Modifiers` has no super field off macOS,
/// so the physical key events are forwarded directly.
pub(super) fn code_str(key: Key) -> Option<&'static str> {
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
pub(super) fn us_char(key: Key, shift: bool) -> Option<char> {
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

/// Modifier code strings the widget synthesizes from [`egui::Modifiers`]
/// diffs (plus the Meta pair forwarded from physical key events).
pub(super) const MOD_SHIFT: &str = "ShiftLeft";
pub(super) const MOD_CTRL: &str = "ControlLeft";
pub(super) const MOD_ALT: &str = "AltLeft";

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code string this widget can emit, paired with a plausible
    /// produced char — from the key table or the modifier synthesizer.
    fn emitted_codes() -> Vec<(&'static str, Option<String>)> {
        let mut codes: Vec<(&'static str, Option<String>)> = Key::ALL
            .iter()
            .filter_map(|&key| Some((code_str(key)?, us_char(key, false).map(String::from))))
            .collect();
        codes.extend(
            [MOD_SHIFT, MOD_CTRL, MOD_ALT, "MetaLeft", "MetaRight"]
                .into_iter()
                .map(|code| (code, None)),
        );
        codes
    }

    /// Every emitted code must resolve through forge-core's VNC keysym
    /// table: a code the engine drops silently is a dead key on the wire.
    #[cfg(feature = "vnc")]
    #[test]
    fn every_emitted_code_resolves_to_a_keysym() {
        use forge_core::widgets::keymap::keysym;
        for (code, produced) in emitted_codes() {
            assert!(
                keysym::keysym(code, produced.as_deref()).is_some(),
                "no keysym for code {code} (key {produced:?})"
            );
        }
    }

    /// Same contract for forge-core's RDP set-1 scancode table.
    #[cfg(feature = "rdp")]
    #[test]
    fn every_emitted_code_resolves_to_a_scancode() {
        use forge_core::widgets::keymap::scancode;
        for (code, _) in emitted_codes() {
            assert!(
                scancode::scancode(code).is_some(),
                "no scancode for code {code}"
            );
        }
    }

    /// F13+ and media keys must stay unmapped: forge-core's US tables stop
    /// at F12, so emitting them would produce dead codes.
    #[test]
    fn unrepresentable_keys_are_skipped() {
        assert_eq!(code_str(Key::F13), None);
        assert_eq!(code_str(Key::F24), None);
        assert_eq!(code_str(Key::Copy), None);
        assert_eq!(code_str(Key::BrowserBack), None);
        // Plain modifiers are synthesized from Modifiers diffs, never mapped.
        assert_eq!(code_str(Key::ShiftLeft), None);
        assert_eq!(code_str(Key::ControlRight), None);
        assert_eq!(code_str(Key::AltLeft), None);
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

    /// The digit/punctuation shift pairs must agree with the VNC keysym
    /// path: the produced char IS the keysym for printables.
    #[cfg(feature = "vnc")]
    #[test]
    fn produced_chars_reach_the_unicode_keysym_path() {
        use forge_core::widgets::keymap::keysym;
        assert_eq!(
            keysym::keysym(
                "Digit1",
                us_char(Key::Num1, true).map(String::from).as_deref()
            ),
            Some('!' as u32)
        );
        assert_eq!(
            keysym::keysym("KeyA", us_char(Key::A, false).map(String::from).as_deref()),
            Some('a' as u32)
        );
    }
}
