//! Keyboard set-1 (PC/AT) scancodes for RDP FastPath input, with the E0
//! extended-key flag. Mapped from the layout-independent `KeyboardEvent.code`.

/// Ctrl+Alt+Del as scancodes: LCtrl, LAlt, Del(extended).
pub const CAD: [(u16, bool); 3] = [(0x1D, false), (0x38, false), (0x53, true)];

/// Resolve `KeyboardEvent.code` → `(set-1 scancode, extended)`.
/// `None` = unmapped (skip the event).
pub fn scancode(code: &str) -> Option<(u16, bool)> {
    let plain = |sc: u16| Some((sc, false));
    let ext = |sc: u16| Some((sc, true));
    match code {
        "Escape" => plain(0x01),
        "Digit1" => plain(0x02),
        "Digit2" => plain(0x03),
        "Digit3" => plain(0x04),
        "Digit4" => plain(0x05),
        "Digit5" => plain(0x06),
        "Digit6" => plain(0x07),
        "Digit7" => plain(0x08),
        "Digit8" => plain(0x09),
        "Digit9" => plain(0x0A),
        "Digit0" => plain(0x0B),
        "Minus" => plain(0x0C),
        "Equal" => plain(0x0D),
        "Backspace" => plain(0x0E),
        "Tab" => plain(0x0F),
        "KeyQ" => plain(0x10),
        "KeyW" => plain(0x11),
        "KeyE" => plain(0x12),
        "KeyR" => plain(0x13),
        "KeyT" => plain(0x14),
        "KeyY" => plain(0x15),
        "KeyU" => plain(0x16),
        "KeyI" => plain(0x17),
        "KeyO" => plain(0x18),
        "KeyP" => plain(0x19),
        "BracketLeft" => plain(0x1A),
        "BracketRight" => plain(0x1B),
        "Enter" => plain(0x1C),
        "ControlLeft" => plain(0x1D),
        "KeyA" => plain(0x1E),
        "KeyS" => plain(0x1F),
        "KeyD" => plain(0x20),
        "KeyF" => plain(0x21),
        "KeyG" => plain(0x22),
        "KeyH" => plain(0x23),
        "KeyJ" => plain(0x24),
        "KeyK" => plain(0x25),
        "KeyL" => plain(0x26),
        "Semicolon" => plain(0x27),
        "Quote" => plain(0x28),
        "Backquote" => plain(0x29),
        "ShiftLeft" => plain(0x2A),
        "Backslash" => plain(0x2B),
        "KeyZ" => plain(0x2C),
        "KeyX" => plain(0x2D),
        "KeyC" => plain(0x2E),
        "KeyV" => plain(0x2F),
        "KeyB" => plain(0x30),
        "KeyN" => plain(0x31),
        "KeyM" => plain(0x32),
        "Comma" => plain(0x33),
        "Period" => plain(0x34),
        "Slash" => plain(0x35),
        "ShiftRight" => plain(0x36),
        "NumpadMultiply" => plain(0x37),
        "AltLeft" => plain(0x38),
        "Space" => plain(0x39),
        "CapsLock" => plain(0x3A),
        "F1" => plain(0x3B),
        "F2" => plain(0x3C),
        "F3" => plain(0x3D),
        "F4" => plain(0x3E),
        "F5" => plain(0x3F),
        "F6" => plain(0x40),
        "F7" => plain(0x41),
        "F8" => plain(0x42),
        "F9" => plain(0x43),
        "F10" => plain(0x44),
        "NumLock" => plain(0x45),
        "ScrollLock" => plain(0x46),
        "Numpad7" => plain(0x47),
        "Numpad8" => plain(0x48),
        "Numpad9" => plain(0x49),
        "NumpadSubtract" => plain(0x4A),
        "Numpad4" => plain(0x4B),
        "Numpad5" => plain(0x4C),
        "Numpad6" => plain(0x4D),
        "NumpadAdd" => plain(0x4E),
        "Numpad1" => plain(0x4F),
        "Numpad2" => plain(0x50),
        "Numpad3" => plain(0x51),
        "Numpad0" => plain(0x52),
        "NumpadDecimal" => plain(0x53),
        "IntlBackslash" => plain(0x56),
        "F11" => plain(0x57),
        "F12" => plain(0x58),
        // Extended (E0-prefixed) keys.
        "ControlRight" => ext(0x1D),
        "AltRight" => ext(0x38),
        "NumpadEnter" => ext(0x1C),
        "NumpadDivide" => ext(0x35),
        "PrintScreen" => ext(0x37),
        "Home" => ext(0x47),
        "ArrowUp" => ext(0x48),
        "PageUp" => ext(0x49),
        "ArrowLeft" => ext(0x4B),
        "ArrowRight" => ext(0x4D),
        "End" => ext(0x4F),
        "ArrowDown" => ext(0x50),
        "PageDown" => ext(0x51),
        "Insert" => ext(0x52),
        "Delete" => ext(0x53),
        "MetaLeft" => ext(0x5B),
        "MetaRight" => ext(0x5C),
        "ContextMenu" => ext(0x5D),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_controls() {
        assert_eq!(scancode("KeyA"), Some((0x1E, false)));
        assert_eq!(scancode("Digit1"), Some((0x02, false)));
        assert_eq!(scancode("Enter"), Some((0x1C, false)));
        assert_eq!(scancode("Space"), Some((0x39, false)));
    }

    #[test]
    fn extended_keys_carry_the_e0_flag() {
        assert_eq!(scancode("ArrowUp"), Some((0x48, true)));
        assert_eq!(scancode("Delete"), Some((0x53, true)));
        assert_eq!(scancode("ControlRight"), Some((0x1D, true)));
        assert_eq!(scancode("NumpadEnter"), Some((0x1C, true)));
    }

    #[test]
    fn numpad_plain_vs_arrows_extended() {
        // Same base scancode, distinguished only by the E0 flag.
        assert_eq!(scancode("Numpad8"), Some((0x48, false)));
        assert_eq!(scancode("ArrowUp"), Some((0x48, true)));
    }

    #[test]
    fn unmapped_returns_none() {
        assert_eq!(scancode("Pause"), None); // E1-prefixed, not set-1 v1
        assert_eq!(scancode("Unknown"), None);
    }

    #[test]
    fn cad_matches_the_table() {
        assert_eq!(
            CAD,
            [
                scancode("ControlLeft").unwrap(),
                scancode("AltLeft").unwrap(),
                scancode("Delete").unwrap(),
            ]
        );
    }
}
