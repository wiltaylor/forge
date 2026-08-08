//! Mouse reports: which events a tracking mode wants, and the bytes each one
//! sends.
//!
//! A program running in the terminal turns tracking on with DECSET (`?1000`
//! and friends) and picks an encoding (`?1005`, `?1006`). Until it does, the
//! kit keeps the pointer for itself — a plain shell reports [`MouseMode::None`],
//! so clicks and scroll stay with the application for focus and selection.
//!
//! Two steps, in this order:
//!
//! 1. [`is_reported`] — does the active mode want this event at all?
//! 2. [`encode`] — the bytes for it, in the active encoding.
//!
//! Both take a [`MouseReport`], which is xterm's own vocabulary: a button
//! code, a motion flag, a release flag, a 0-based cell and the modifiers.
//! Each kit maps its own event type onto that.

/// The tracking mode the running program asked for.
///
/// Mirrors the DECSET modes an emulator reports: `?9` is [`MouseMode::Press`],
/// `?1000` is [`MouseMode::PressRelease`], `?1002` is
/// [`MouseMode::ButtonMotion`] and `?1003` is [`MouseMode::AnyMotion`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MouseMode {
    /// Tracking is off. Nothing is reported.
    #[default]
    None,
    /// Button presses only (X10).
    Press,
    /// Presses and releases (VT200).
    PressRelease,
    /// Presses, releases, and motion while a button is held.
    ButtonMotion,
    /// Presses, releases, and all motion.
    AnyMotion,
}

/// How the report is written on the wire.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MouseEncoding {
    /// One printable byte per field (X10). Coordinates stop at 223 cells.
    #[default]
    Default,
    /// The `?1005` form: each field is a UTF-8 code point, so coordinates
    /// pass the 223-cell wall.
    Utf8,
    /// The `?1006` form: `ESC [ < cb ; col ; row M`, decimal and unbounded,
    /// with a distinct final byte for release.
    Sgr,
}

// The button codes: the low bits of `cb`, before the modifier and motion bits.
// A kit maps its own button type onto these.

/// The left (primary) button.
pub const BUTTON_LEFT: u16 = 0;
/// The middle button.
pub const BUTTON_MIDDLE: u16 = 1;
/// The right (secondary) button.
pub const BUTTON_RIGHT: u16 = 2;
/// No button held — bare motion, and the release code in the byte forms.
pub const BUTTON_NONE: u16 = 3;
/// Wheel up. Wheel codes start here, which is how [`is_reported`] knows one.
pub const WHEEL_UP: u16 = 64;
/// Wheel down.
pub const WHEEL_DOWN: u16 = 65;
/// Wheel left (horizontal scroll).
pub const WHEEL_LEFT: u16 = 66;
/// Wheel right (horizontal scroll).
pub const WHEEL_RIGHT: u16 = 67;

/// The modifier keys held when the event happened.
///
/// A named type, because the three flags always travel together and a kit
/// building them from its own modifier set would otherwise pass three bare
/// booleans — where swapping two of them still compiles and still encodes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    /// Shift was held: `cb` bit 2.
    pub shift: bool,
    /// Alt was held: `cb` bit 3.
    pub alt: bool,
    /// Ctrl was held: `cb` bit 4.
    pub ctrl: bool,
}

impl Modifiers {
    /// No modifier held.
    pub const NONE: Modifiers = Modifiers {
        shift: false,
        alt: false,
        ctrl: false,
    };
}

/// One event to report, in xterm's vocabulary.
///
/// `col` and `row` are 0-based cells within the terminal grid; the wire form
/// is 1-based and [`encode`] does that step. A kit subtracts its pane origin
/// before it builds the report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MouseReport {
    /// The base button code: [`BUTTON_LEFT`], [`BUTTON_NONE`], a `WHEEL_*`.
    pub button: u16,
    /// The pointer moved into this cell (drag, or bare motion).
    pub motion: bool,
    /// The button came up.
    pub release: bool,
    /// 0-based column.
    pub col: u16,
    /// 0-based row.
    pub row: u16,
    /// The modifier keys held.
    pub modifiers: Modifiers,
}

impl MouseReport {
    /// A button going down, or a wheel step — the wheel is a press.
    pub const fn press(button: u16, col: u16, row: u16) -> MouseReport {
        MouseReport {
            button,
            motion: false,
            release: false,
            col,
            row,
            modifiers: Modifiers::NONE,
        }
    }

    /// A button coming up. `button` must be a real button: the wheel has no
    /// release, and [`is_reported`] drops one that claims otherwise.
    pub const fn release(button: u16, col: u16, row: u16) -> MouseReport {
        MouseReport {
            release: true,
            ..MouseReport::press(button, col, row)
        }
    }

    /// The pointer crossing into a cell with `button` held.
    pub const fn drag(button: u16, col: u16, row: u16) -> MouseReport {
        MouseReport {
            motion: true,
            ..MouseReport::press(button, col, row)
        }
    }

    /// The pointer crossing into a cell with no button held.
    pub const fn motion(col: u16, row: u16) -> MouseReport {
        MouseReport::drag(BUTTON_NONE, col, row)
    }

    /// The same report with the modifier keys set.
    pub const fn with_modifiers(self, modifiers: Modifiers) -> MouseReport {
        MouseReport { modifiers, ..self }
    }

    /// A wheel step rather than a button. The four wheel codes are the whole
    /// range — xterm puts the extra buttons at 128 and up, not above 67.
    const fn is_wheel(&self) -> bool {
        self.button >= WHEEL_UP && self.button <= WHEEL_RIGHT
    }

    /// `cb`: the button code with the modifier and motion bits set. The bits
    /// are flags, so an out-of-range button cannot carry into them.
    const fn cb(&self) -> u16 {
        let mut cb = self.button;
        if self.modifiers.shift {
            cb |= 4;
        }
        if self.modifiers.alt {
            cb |= 8;
        }
        if self.modifiers.ctrl {
            cb |= 16;
        }
        if self.motion {
            cb |= 32;
        }
        cb
    }
}

/// Does the active tracking mode want this event? Mirrors xterm: `Press`
/// reports button-down and wheel only; `PressRelease` adds releases;
/// `ButtonMotion` adds drags (motion with a button); `AnyMotion` adds bare
/// motion. The wheel is a press, so every mode except `None` reports it.
///
/// Call this before [`encode`] and drop the event when it returns false.
pub fn is_reported(report: &MouseReport, mode: MouseMode) -> bool {
    let wheel = report.is_wheel();
    // A wheel step has no release — xterm never sends one, and the byte forms
    // would report it as a horizontal scroll, since they carry the release in
    // the button id.
    if wheel && report.release {
        return false;
    }
    match mode {
        MouseMode::None => false,
        MouseMode::Press => !report.release && (!report.motion || wheel),
        MouseMode::PressRelease => !report.motion || wheel,
        MouseMode::ButtonMotion => wheel || !(report.motion && report.button == BUTTON_NONE),
        MouseMode::AnyMotion => true,
    }
}

/// The bytes for `report` in `encoding`.
///
/// Coordinates go out 1-based. The two byte forms have no release code, so a
/// release drops the button id to [`BUTTON_NONE`] there and keeps its
/// modifier bits; only [`MouseEncoding::Sgr`] says which button came up.
pub fn encode(report: &MouseReport, encoding: MouseEncoding) -> Vec<u8> {
    let cb = report.cb();
    match encoding {
        MouseEncoding::Sgr => {
            let final_byte = if report.release { 'm' } else { 'M' };
            format!(
                "\x1b[<{};{};{}{}",
                cb,
                report.col.saturating_add(1),
                report.row.saturating_add(1),
                final_byte
            )
            .into_bytes()
        }
        MouseEncoding::Utf8 => {
            let cb = fold_release(cb, report.release);
            let mut out = vec![0x1b, b'[', b'M'];
            push_utf8(&mut out, cb + 32);
            push_utf8(&mut out, report.col.saturating_add(33));
            push_utf8(&mut out, report.row.saturating_add(33));
            out
        }
        // X10: one printable byte per field, saturating at 255.
        MouseEncoding::Default => {
            let cb = fold_release(cb, report.release);
            vec![
                0x1b,
                b'[',
                b'M',
                (cb + 32).min(255) as u8,
                report.col.saturating_add(33).min(255) as u8,
                report.row.saturating_add(33).min(255) as u8,
            ]
        }
    }
}

/// The byte forms carry no release code, so a release reports button "none"
/// while the modifier and motion bits stay as they are.
const fn fold_release(cb: u16, release: bool) -> u16 {
    if release {
        (cb & !0b11) | BUTTON_NONE
    } else {
        cb
    }
}

/// Append `v` as a UTF-8 code point (the `?1005` encoding widens coords past
/// the 223-cell wall the single-byte form hits).
fn push_utf8(out: &mut Vec<u8>, v: u16) {
    let ch = char::from_u32(v as u32).unwrap_or('\u{fffd}');
    let mut buf = [0u8; 4];
    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
}
