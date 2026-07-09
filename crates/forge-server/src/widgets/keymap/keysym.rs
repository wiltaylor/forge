//! X11 keysyms for the VNC KeyEvent message (RFC 6143 §7.5.4).
//!
//! Non-printables map from `KeyboardEvent.code`; printables use the produced
//! character from `key` — Latin-1 codepoints are keysyms directly, everything
//! else uses the Unicode keysym range (`0x0100_0000 + codepoint`).

/// Ctrl+Alt+Del as keysyms: Control_L, Alt_L, Delete.
pub const CAD: [u32; 3] = [0xFFE3, 0xFFE9, 0xFFFF];

/// Resolve a browser key event to an X11 keysym. `None` = unmapped (skip).
pub fn keysym(code: &str, key: Option<&str>) -> Option<u32> {
    if let Some(sym) = special(code) {
        return Some(sym);
    }
    let mut chars = key?.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        // Multi-char `key` (e.g. "Dead") with an unmapped `code` — skip.
        return None;
    }
    let cp = c as u32;
    Some(if cp <= 0xFF { cp } else { 0x0100_0000 + cp })
}

/// `KeyboardEvent.code` → keysym for non-printable keys.
fn special(code: &str) -> Option<u32> {
    Some(match code {
        "Escape" => 0xFF1B,
        "F1" => 0xFFBE,
        "F2" => 0xFFBF,
        "F3" => 0xFFC0,
        "F4" => 0xFFC1,
        "F5" => 0xFFC2,
        "F6" => 0xFFC3,
        "F7" => 0xFFC4,
        "F8" => 0xFFC5,
        "F9" => 0xFFC6,
        "F10" => 0xFFC7,
        "F11" => 0xFFC8,
        "F12" => 0xFFC9,
        "PrintScreen" => 0xFF61,
        "ScrollLock" => 0xFF14,
        "Pause" => 0xFF13,
        "Backspace" => 0xFF08,
        "Tab" => 0xFF09,
        "Enter" => 0xFF0D,
        "NumpadEnter" => 0xFF8D,
        "CapsLock" => 0xFFE5,
        "ShiftLeft" => 0xFFE1,
        "ShiftRight" => 0xFFE2,
        "ControlLeft" => 0xFFE3,
        "ControlRight" => 0xFFE4,
        "AltLeft" => 0xFFE9,
        "AltRight" => 0xFFEA,
        "MetaLeft" => 0xFFEB,
        "MetaRight" => 0xFFEC,
        "ContextMenu" => 0xFF67,
        "Insert" => 0xFF63,
        "Delete" => 0xFFFF,
        "Home" => 0xFF50,
        "End" => 0xFF57,
        "PageUp" => 0xFF55,
        "PageDown" => 0xFF56,
        "ArrowLeft" => 0xFF51,
        "ArrowUp" => 0xFF52,
        "ArrowRight" => 0xFF53,
        "ArrowDown" => 0xFF54,
        "NumLock" => 0xFF7F,
        "NumpadDivide" => 0xFFAF,
        "NumpadMultiply" => 0xFFAA,
        "NumpadSubtract" => 0xFFAD,
        "NumpadAdd" => 0xFFAB,
        "NumpadDecimal" => 0xFFAE,
        "Numpad0" => 0xFFB0,
        "Numpad1" => 0xFFB1,
        "Numpad2" => 0xFFB2,
        "Numpad3" => 0xFFB3,
        "Numpad4" => 0xFFB4,
        "Numpad5" => 0xFFB5,
        "Numpad6" => 0xFFB6,
        "Numpad7" => 0xFFB7,
        "Numpad8" => 0xFFB8,
        "Numpad9" => 0xFFB9,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_printables_use_the_code_table() {
        assert_eq!(keysym("Enter", Some("Enter")), Some(0xFF0D));
        assert_eq!(keysym("ArrowUp", None), Some(0xFF52));
        assert_eq!(keysym("ShiftLeft", Some("Shift")), Some(0xFFE1));
        assert_eq!(keysym("Delete", Some("Delete")), Some(0xFFFF));
    }

    #[test]
    fn printables_use_the_produced_character() {
        assert_eq!(keysym("KeyA", Some("a")), Some('a' as u32));
        assert_eq!(keysym("KeyA", Some("A")), Some('A' as u32));
        assert_eq!(keysym("Digit1", Some("!")), Some('!' as u32));
        assert_eq!(keysym("Space", Some(" ")), Some(0x20));
        // Latin-1 high half maps directly.
        assert_eq!(keysym("KeyE", Some("é")), Some(0xE9));
    }

    #[test]
    fn non_latin1_uses_unicode_keysyms() {
        assert_eq!(keysym("KeyC", Some("č")), Some(0x0100_0000 + 'č' as u32));
        assert_eq!(keysym("KeyJ", Some("あ")), Some(0x0100_0000 + 'あ' as u32));
    }

    #[test]
    fn unmapped_keys_are_skipped() {
        assert_eq!(keysym("Dead", Some("Dead")), None);
        assert_eq!(keysym("KeyA", None), None);
        assert_eq!(keysym("Unknown", Some("")), None);
    }

    #[test]
    fn cad_is_ctrl_alt_del() {
        assert_eq!(
            CAD,
            [
                keysym("ControlLeft", None).unwrap(),
                keysym("AltLeft", None).unwrap(),
                keysym("Delete", None).unwrap(),
            ]
        );
    }
}
