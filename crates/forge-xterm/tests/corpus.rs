//! The mouse-report corpus: an event plus modifiers plus mode, mapped to the
//! exact bytes the running program must receive.
//!
//! Both kits call the same encoder, so one table covers both. Add a case here
//! when a kit meets one — not to that kit's own tests, where the other kit
//! would never see it. `want: None` means the mode does not report the event
//! at all, which is the gating half of the table.
//!
//! The bytes are the whole contract. This is a wire protocol, so nothing else
//! is worth asserting on.

use forge_xterm::mouse::{
    encode, is_reported, Modifiers, MouseEncoding, MouseMode, MouseReport, BUTTON_LEFT,
    BUTTON_MIDDLE, BUTTON_NONE, BUTTON_RIGHT, WHEEL_DOWN, WHEEL_LEFT, WHEEL_RIGHT, WHEEL_UP,
};

struct Case {
    /// What the row pins, in words. Names the failure when it fails.
    name: &'static str,
    report: MouseReport,
    mode: MouseMode,
    encoding: MouseEncoding,
    /// The bytes, or `None` when `mode` does not report the event.
    want: Option<&'static [u8]>,
}

const SGR: MouseEncoding = MouseEncoding::Sgr;
const X10: MouseEncoding = MouseEncoding::Default;
const UTF8: MouseEncoding = MouseEncoding::Utf8;

const CORPUS: &[Case] = &[
    // ---- SGR (?1006): decimal fields, distinct final byte for release. ----
    Case {
        name: "sgr left press at the origin cell reports 1-based coords",
        report: MouseReport::press(BUTTON_LEFT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<0;1;1M"),
    },
    Case {
        name: "sgr release keeps the button id and ends in lowercase m",
        report: MouseReport::release(BUTTON_LEFT, 4, 2),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<0;5;3m"),
    },
    Case {
        name: "sgr middle press",
        report: MouseReport::press(BUTTON_MIDDLE, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<1;1;1M"),
    },
    Case {
        name: "sgr right drag adds the motion bit and keeps the button id",
        report: MouseReport::drag(BUTTON_RIGHT, 9, 1),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<34;10;2M"),
    },
    Case {
        name: "sgr bare motion is button none plus the motion bit",
        report: MouseReport::motion(0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<35;1;1M"),
    },
    Case {
        name: "sgr coordinates are decimal and are not capped",
        report: MouseReport::press(BUTTON_LEFT, 300, 400),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<0;301;401M"),
    },
    // ---- Modifiers: shift +4, alt +8, ctrl +16. ----
    Case {
        name: "shift adds 4",
        report: MouseReport::press(BUTTON_LEFT, 0, 0).with_modifiers(Modifiers {
            shift: true,
            alt: false,
            ctrl: false,
        }),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<4;1;1M"),
    },
    Case {
        name: "alt adds 8",
        report: MouseReport::press(BUTTON_LEFT, 0, 0).with_modifiers(Modifiers {
            shift: false,
            alt: true,
            ctrl: false,
        }),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<8;1;1M"),
    },
    Case {
        name: "ctrl adds 16",
        report: MouseReport::press(BUTTON_LEFT, 0, 0).with_modifiers(Modifiers {
            shift: false,
            alt: false,
            ctrl: true,
        }),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<16;1;1M"),
    },
    Case {
        name: "ctrl and shift together add 20",
        report: MouseReport::press(BUTTON_LEFT, 0, 0).with_modifiers(Modifiers {
            shift: true,
            alt: false,
            ctrl: true,
        }),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<20;1;1M"),
    },
    Case {
        name: "all three modifiers on a drag add 28 plus the motion bit",
        report: MouseReport::drag(BUTTON_LEFT, 0, 0).with_modifiers(Modifiers {
            shift: true,
            alt: true,
            ctrl: true,
        }),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<60;1;1M"),
    },
    // ---- Wheel: codes 64 to 67, reported as presses. ----
    Case {
        name: "wheel up is a press with code 64",
        report: MouseReport::press(WHEEL_UP, 3, 3),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<64;4;4M"),
    },
    Case {
        name: "wheel down is code 65",
        report: MouseReport::press(WHEEL_DOWN, 3, 3),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<65;4;4M"),
    },
    Case {
        name: "wheel left is code 66",
        report: MouseReport::press(WHEEL_LEFT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<66;1;1M"),
    },
    Case {
        name: "wheel right is code 67",
        report: MouseReport::press(WHEEL_RIGHT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<67;1;1M"),
    },
    Case {
        name: "a modifier on the wheel keeps the wheel code",
        report: MouseReport::press(WHEEL_UP, 0, 0).with_modifiers(Modifiers {
            shift: false,
            alt: false,
            ctrl: true,
        }),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<80;1;1M"),
    },
    Case {
        name: "the wheel has no release, so a mode that reports releases drops one",
        report: MouseReport::release(WHEEL_UP, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: None,
    },
    // ---- Default (X10): ESC [ M then three printable bytes. ----
    Case {
        name: "x10 left press at the origin is 32, 33, 33",
        report: MouseReport::press(BUTTON_LEFT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: X10,
        want: Some(&[0x1b, b'[', b'M', 32, 33, 33]),
    },
    Case {
        name: "x10 has no release code, so a release drops the button id to none",
        report: MouseReport::release(BUTTON_RIGHT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: X10,
        want: Some(&[0x1b, b'[', b'M', 35, 33, 33]),
    },
    Case {
        name: "x10 release keeps the modifier bits it drops the button id from",
        report: MouseReport::release(BUTTON_RIGHT, 0, 0).with_modifiers(Modifiers {
            shift: false,
            alt: false,
            ctrl: true,
        }),
        mode: MouseMode::AnyMotion,
        encoding: X10,
        want: Some(&[0x1b, b'[', b'M', 51, 33, 33]),
    },
    Case {
        name: "x10 drag adds the motion bit",
        report: MouseReport::drag(BUTTON_LEFT, 1, 1),
        mode: MouseMode::AnyMotion,
        encoding: X10,
        want: Some(&[0x1b, b'[', b'M', 64, 34, 34]),
    },
    Case {
        name: "x10 coordinates saturate at one byte",
        report: MouseReport::press(BUTTON_LEFT, 300, 400),
        mode: MouseMode::AnyMotion,
        encoding: X10,
        want: Some(&[0x1b, b'[', b'M', 32, 255, 255]),
    },
    // ---- UTF-8 (?1005): same fields, widened past the 223-cell wall. ----
    Case {
        name: "utf8 matches x10 while every field stays under 128",
        report: MouseReport::press(BUTTON_LEFT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: UTF8,
        want: Some(&[0x1b, b'[', b'M', 32, 33, 33]),
    },
    Case {
        name: "utf8 widens a column the single-byte form would have capped",
        report: MouseReport::press(BUTTON_LEFT, 200, 0),
        mode: MouseMode::AnyMotion,
        encoding: UTF8,
        want: Some(&[0x1b, b'[', b'M', 32, 0xc3, 0xa9, 33]),
    },
    Case {
        name: "utf8 has no release code either",
        report: MouseReport::release(BUTTON_RIGHT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: UTF8,
        want: Some(&[0x1b, b'[', b'M', 35, 33, 33]),
    },
    // ---- Mode gating: tracking off. ----
    Case {
        name: "tracking off drops a press",
        report: MouseReport::press(BUTTON_LEFT, 0, 0),
        mode: MouseMode::None,
        encoding: SGR,
        want: None,
    },
    Case {
        name: "tracking off drops the wheel, so the app keeps its scroll",
        report: MouseReport::press(WHEEL_UP, 0, 0),
        mode: MouseMode::None,
        encoding: SGR,
        want: None,
    },
    // ---- Mode gating: press only (?9). ----
    Case {
        name: "press mode reports a press",
        report: MouseReport::press(BUTTON_LEFT, 0, 0),
        mode: MouseMode::Press,
        encoding: SGR,
        want: Some(b"\x1b[<0;1;1M"),
    },
    Case {
        name: "press mode reports the wheel",
        report: MouseReport::press(WHEEL_UP, 0, 0),
        mode: MouseMode::Press,
        encoding: SGR,
        want: Some(b"\x1b[<64;1;1M"),
    },
    Case {
        name: "press mode drops a release",
        report: MouseReport::release(BUTTON_LEFT, 0, 0),
        mode: MouseMode::Press,
        encoding: SGR,
        want: None,
    },
    Case {
        name: "press mode drops a drag",
        report: MouseReport::drag(BUTTON_LEFT, 0, 0),
        mode: MouseMode::Press,
        encoding: SGR,
        want: None,
    },
    Case {
        name: "press mode drops bare motion",
        report: MouseReport::motion(0, 0),
        mode: MouseMode::Press,
        encoding: SGR,
        want: None,
    },
    // ---- Mode gating: press and release (?1000). ----
    Case {
        name: "press-release mode reports a release",
        report: MouseReport::release(BUTTON_LEFT, 0, 0),
        mode: MouseMode::PressRelease,
        encoding: SGR,
        want: Some(b"\x1b[<0;1;1m"),
    },
    Case {
        name: "press-release mode reports the wheel",
        report: MouseReport::press(WHEEL_DOWN, 0, 0),
        mode: MouseMode::PressRelease,
        encoding: SGR,
        want: Some(b"\x1b[<65;1;1M"),
    },
    Case {
        name: "press-release mode drops a drag",
        report: MouseReport::drag(BUTTON_LEFT, 0, 0),
        mode: MouseMode::PressRelease,
        encoding: SGR,
        want: None,
    },
    Case {
        name: "press-release mode drops bare motion",
        report: MouseReport::motion(0, 0),
        mode: MouseMode::PressRelease,
        encoding: SGR,
        want: None,
    },
    // ---- Mode gating: button motion (?1002). ----
    Case {
        name: "button-motion mode reports a drag",
        report: MouseReport::drag(BUTTON_LEFT, 0, 0),
        mode: MouseMode::ButtonMotion,
        encoding: SGR,
        want: Some(b"\x1b[<32;1;1M"),
    },
    Case {
        name: "button-motion mode reports a release",
        report: MouseReport::release(BUTTON_LEFT, 0, 0),
        mode: MouseMode::ButtonMotion,
        encoding: SGR,
        want: Some(b"\x1b[<0;1;1m"),
    },
    Case {
        name: "button-motion mode reports the wheel",
        report: MouseReport::press(WHEEL_UP, 0, 0),
        mode: MouseMode::ButtonMotion,
        encoding: SGR,
        want: Some(b"\x1b[<64;1;1M"),
    },
    Case {
        name: "button-motion mode drops motion with no button held",
        report: MouseReport::motion(0, 0),
        mode: MouseMode::ButtonMotion,
        encoding: SGR,
        want: None,
    },
    // ---- Mode gating: any motion (?1003). ----
    Case {
        name: "any-motion mode reports bare motion",
        report: MouseReport::motion(0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<35;1;1M"),
    },
    Case {
        name: "any-motion mode reports a release",
        report: MouseReport::release(BUTTON_LEFT, 0, 0),
        mode: MouseMode::AnyMotion,
        encoding: SGR,
        want: Some(b"\x1b[<0;1;1m"),
    },
];

#[test]
fn corpus_holds() {
    for case in CORPUS {
        let reported = is_reported(&case.report, case.mode);
        match case.want {
            None => assert!(
                !reported,
                "{}: expected the mode to drop the event",
                case.name
            ),
            Some(want) => {
                assert!(
                    reported,
                    "{}: expected the mode to report the event",
                    case.name
                );
                let got = encode(&case.report, case.encoding);
                assert_eq!(got, want, "{}", case.name);
            }
        }
    }
}

/// Every mode and every encoding must appear in the corpus. Adding a variant
/// breaks the encoder's `match` first; this stops the variant landing with no
/// case to pin its bytes.
#[test]
fn corpus_covers_every_mode_and_encoding() {
    let modes = [
        MouseMode::None,
        MouseMode::Press,
        MouseMode::PressRelease,
        MouseMode::ButtonMotion,
        MouseMode::AnyMotion,
    ];
    for mode in modes {
        assert!(
            CORPUS.iter().any(|c| c.mode == mode),
            "no corpus case for {mode:?}"
        );
    }
    let encodings = [
        MouseEncoding::Default,
        MouseEncoding::Utf8,
        MouseEncoding::Sgr,
    ];
    for encoding in encodings {
        assert!(
            CORPUS
                .iter()
                .any(|c| c.encoding == encoding && c.want.is_some()),
            "no corpus case for {encoding:?}"
        );
    }
}

/// The button and wheel codes are the vocabulary each kit maps its own button
/// type onto. Pin them, because a kit's adapter is written against the values.
#[test]
fn button_codes_are_the_xterm_ones() {
    assert_eq!(
        [BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_RIGHT, BUTTON_NONE],
        [0, 1, 2, 3]
    );
    assert_eq!(
        [WHEEL_UP, WHEEL_DOWN, WHEEL_LEFT, WHEEL_RIGHT],
        [64, 65, 66, 67]
    );
}
