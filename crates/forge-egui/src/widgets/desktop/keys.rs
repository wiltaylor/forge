//! egui key → browser-style key event fields for the desktop wire protocol.
//!
//! The protocol ([`forge_core::widgets::proto::DesktopClientMsg::Key`]) wants
//! `KeyboardEvent.code` (layout-independent physical key) plus the produced
//! character in `key`. That pair comes off [`crate::keys`], the crate's one
//! US-layout bridge — the block editor's key adapter reads the same table.
//!
//! Every string returned by [`code_str`] must resolve through **both**
//! forge-core keymaps (`keysym` for VNC, `scancode` for RDP) — enforced by
//! the tests below. That is why F13+ and media keys map to `None`: they'd be
//! dead codes on the wire.

pub(super) use crate::keys::{code_str, us_char};

/// Modifier code strings the widget synthesizes from [`egui::Modifiers`]
/// diffs (plus the Meta pair forwarded from physical key events).
pub(super) const MOD_SHIFT: &str = "ShiftLeft";
pub(super) const MOD_CTRL: &str = "ControlLeft";
pub(super) const MOD_ALT: &str = "AltLeft";

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key;

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
