//! forge-xterm — one home for the terminal wire encodings.
//!
//! The kits (forge-tui, forge-egui) each drive a terminal emulator and must
//! turn user input into the bytes xterm-compatible programs expect. Those
//! bytes are a protocol, not a rendering choice, so they live here once
//! instead of in each kit.
//!
//! The crate is synchronous and has no dependencies. forge-tui has no async
//! runtime and does not want one; keeping this crate free of dependencies is
//! what lets both kits share the encodings without the terminal engines
//! having to merge.
//!
//! Two encoders, one vocabulary: [`key`] turns a key press into bytes, and
//! [`mouse`] turns a pointer event into a report. Both take [`Modifiers`].
//!
//! Each kit adapts its own input types (crossterm's, egui's) to the vocabulary
//! here — see [`key::Key`] and [`mouse::MouseReport`]. The authored corpora in
//! `tests/key_corpus.rs` and `tests/mouse_corpus.rs` pin every key, event,
//! modifier and mode to its exact bytes, so a case added for one kit is
//! checked for both.

pub mod key;
pub mod mouse;

/// The modifier keys held when the event happened.
///
/// A named type, because the three flags always travel together and a kit
/// building them from its own modifier set would otherwise pass three bare
/// booleans — where swapping two of them still compiles and still encodes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    /// Shift was held.
    pub shift: bool,
    /// Alt (meta) was held.
    pub alt: bool,
    /// Ctrl was held.
    pub ctrl: bool,
}

impl Modifiers {
    /// No modifier held.
    pub const NONE: Modifiers = Modifiers {
        shift: false,
        alt: false,
        ctrl: false,
    };
    /// Shift alone.
    pub const SHIFT: Modifiers = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    /// Alt alone.
    pub const ALT: Modifiers = Modifiers {
        alt: true,
        ..Modifiers::NONE
    };
    /// Ctrl alone.
    pub const CTRL: Modifiers = Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    };
}

/// Append the code point `code` to `out` as UTF-8.
///
/// Both encoders need it: the mouse `?1005` form writes each field as a code
/// point to pass the 223-cell wall the single-byte form hits, and a character
/// key sends the character itself. A value that is not a code point (a
/// surrogate, or past the Unicode range) becomes U+FFFD rather than nothing,
/// so a field keeps its place in the sequence.
pub fn push_utf8(out: &mut Vec<u8>, code: u32) {
    let ch = char::from_u32(code).unwrap_or('\u{fffd}');
    let mut buf = [0u8; 4];
    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
}

#[cfg(test)]
mod tests {
    use super::push_utf8;

    /// The two corpora exercise the helper through the encoders. This pins the
    /// one branch neither reaches: a value that is not a code point.
    #[test]
    fn push_utf8_appends_the_code_point_or_the_replacement() {
        let mut out = Vec::new();
        push_utf8(&mut out, u32::from(b'a'));
        push_utf8(&mut out, 0xe9); // é
        assert_eq!(out, vec![b'a', 0xc3, 0xa9]);

        // A surrogate is not a code point, and neither is anything past the
        // Unicode range: both become U+FFFD, so the field keeps its place.
        let mut out = Vec::new();
        push_utf8(&mut out, 0xd800);
        push_utf8(&mut out, 0x11_0000);
        assert_eq!(out, "\u{fffd}\u{fffd}".as_bytes());
    }
}
