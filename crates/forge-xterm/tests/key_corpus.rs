//! The key corpus: a key plus its modifiers plus the cursor-key mode, mapped
//! to the exact bytes the running program must receive.
//!
//! Both kits call the same table, so one corpus covers both. Add a case here
//! when a kit meets one — not to that kit's own tests, where the other kit
//! would never see it. `want: None` means the key has no representation, and
//! the kit must send nothing rather than a plausible-looking wrong code.
//!
//! The bytes are the whole contract. This is a wire protocol, so nothing else
//! is worth asserting on.

use forge_xterm::key::{encode, CursorKeys, Key};
use forge_xterm::Modifiers;

struct Case {
    /// What the row pins, in words. Names the failure when it fails.
    name: &'static str,
    key: Key,
    modifiers: Modifiers,
    cursor: CursorKeys,
    /// The bytes, or `None` when the key has no representation.
    want: Option<&'static [u8]>,
}

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;
const CTRL: Modifiers = Modifiers::CTRL;
const NORMAL: CursorKeys = CursorKeys::Normal;
const APP: CursorKeys = CursorKeys::Application;

const CORPUS: &[Case] = &[
    // ---- Characters: the key sends what it prints. ----
    Case {
        name: "a plain letter sends itself",
        key: Key::Char('a'),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"a"),
    },
    Case {
        name: "shift does not change the bytes — the kit decides which character was typed",
        key: Key::Char('a'),
        modifiers: SHIFT,
        cursor: NORMAL,
        want: Some(b"a"),
    },
    Case {
        name: "a non-ascii character goes out as utf-8, not as a truncated byte",
        key: Key::Char('é'),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(&[0xc3, 0xa9]),
    },
    // ---- Ctrl chords: the control byte, not the letter. ----
    Case {
        name: "ctrl+a is 0x01",
        key: Key::Char('a'),
        modifiers: CTRL,
        cursor: NORMAL,
        want: Some(&[0x01]),
    },
    Case {
        name: "ctrl+c is 0x03, the interrupt every shell expects",
        key: Key::Char('c'),
        modifiers: CTRL,
        cursor: NORMAL,
        want: Some(&[0x03]),
    },
    Case {
        name: "an upper-case letter gives the same control byte",
        key: Key::Char('C'),
        modifiers: CTRL,
        cursor: NORMAL,
        want: Some(&[0x03]),
    },
    Case {
        name: "ctrl+shift+c is still 0x03 — shift adds no bit here",
        key: Key::Char('c'),
        modifiers: Modifiers {
            shift: true,
            alt: false,
            ctrl: true,
        },
        cursor: NORMAL,
        want: Some(&[0x03]),
    },
    Case {
        name: "ctrl on a digit has no control byte, so it sends nothing",
        key: Key::Char('1'),
        modifiers: CTRL,
        cursor: NORMAL,
        want: None,
    },
    // ---- Alt chords: the readline meta prefix. ----
    Case {
        name: "alt+x is ESC then the character",
        key: Key::Char('x'),
        modifiers: ALT,
        cursor: NORMAL,
        want: Some(&[0x1b, b'x']),
    },
    Case {
        name: "alt on a non-ascii character keeps the utf-8 form after the ESC",
        key: Key::Char('é'),
        modifiers: ALT,
        cursor: NORMAL,
        want: Some(&[0x1b, 0xc3, 0xa9]),
    },
    Case {
        name: "ctrl and alt compose: ESC then the control byte",
        key: Key::Char('x'),
        modifiers: Modifiers {
            shift: false,
            alt: true,
            ctrl: true,
        },
        cursor: NORMAL,
        want: Some(&[0x1b, 0x18]),
    },
    Case {
        name: "ctrl and alt on a digit still send nothing — there is no control byte to prefix",
        key: Key::Char('1'),
        modifiers: Modifiers {
            shift: false,
            alt: true,
            ctrl: true,
        },
        cursor: NORMAL,
        want: None,
    },
    // ---- The editing keys. ----
    Case {
        name: "enter is carriage return, not newline",
        key: Key::Enter,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\r"),
    },
    Case {
        name: "backspace is DEL",
        key: Key::Backspace,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(&[0x7f]),
    },
    Case {
        name: "tab is a tab byte",
        key: Key::Tab,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\t"),
    },
    Case {
        name: "escape is a bare ESC",
        key: Key::Escape,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(&[0x1b]),
    },
    // ---- The cursor keys, normal mode: CSI. ----
    Case {
        name: "up is CSI A while the cursor keys are normal",
        key: Key::Up,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[A"),
    },
    Case {
        name: "down is CSI B",
        key: Key::Down,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[B"),
    },
    Case {
        name: "right is CSI C",
        key: Key::Right,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[C"),
    },
    Case {
        name: "left is CSI D",
        key: Key::Left,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[D"),
    },
    // ---- The cursor keys, application mode (DECCKM ?1h): SS3. ----
    Case {
        name: "application mode sends up as SS3 A",
        key: Key::Up,
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOA"),
    },
    Case {
        name: "application mode sends down as SS3 B",
        key: Key::Down,
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOB"),
    },
    Case {
        name: "application mode sends right as SS3 C",
        key: Key::Right,
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOC"),
    },
    Case {
        name: "application mode sends left as SS3 D",
        key: Key::Left,
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOD"),
    },
    Case {
        name: "a modifier on a cursor key is dropped — the plain sequence goes out",
        key: Key::Up,
        modifiers: CTRL,
        cursor: NORMAL,
        want: Some(b"\x1b[A"),
    },
    // Home and End are cursor keys too: terminfo's `khome` under `smkx` is
    // `\EOH`, so a full-screen program in application mode expects SS3.
    Case {
        name: "home is CSI H while the cursor keys are normal",
        key: Key::Home,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[H"),
    },
    Case {
        name: "application mode sends home as SS3 H",
        key: Key::Home,
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOH"),
    },
    Case {
        name: "end is CSI F",
        key: Key::End,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[F"),
    },
    Case {
        name: "application mode sends end as SS3 F",
        key: Key::End,
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOF"),
    },
    // ---- Navigation and editing: the tilde sequences. ----
    Case {
        name: "page up is CSI 5 tilde",
        key: Key::PageUp,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[5~"),
    },
    Case {
        name: "page down is CSI 6 tilde",
        key: Key::PageDown,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[6~"),
    },
    Case {
        name: "insert is CSI 2 tilde",
        key: Key::Insert,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[2~"),
    },
    Case {
        name: "delete is CSI 3 tilde",
        key: Key::Delete,
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[3~"),
    },
    // ---- The function keys: F1 to F4 are SS3, F5 up are tilde sequences. ----
    Case {
        name: "f1 is SS3 P",
        key: Key::Function(1),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1bOP"),
    },
    Case {
        name: "f2 is SS3 Q",
        key: Key::Function(2),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1bOQ"),
    },
    Case {
        name: "f3 is SS3 R",
        key: Key::Function(3),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1bOR"),
    },
    Case {
        name: "f4 is SS3 S",
        key: Key::Function(4),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1bOS"),
    },
    Case {
        name: "f1 keeps its SS3 form in application cursor mode",
        key: Key::Function(1),
        modifiers: NONE,
        cursor: APP,
        want: Some(b"\x1bOP"),
    },
    Case {
        name: "f5 is CSI 15 tilde",
        key: Key::Function(5),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[15~"),
    },
    Case {
        name: "f6 is CSI 17 tilde — 16 is skipped, as xterm skips it",
        key: Key::Function(6),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[17~"),
    },
    Case {
        name: "f7 is CSI 18 tilde",
        key: Key::Function(7),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[18~"),
    },
    Case {
        name: "f8 is CSI 19 tilde",
        key: Key::Function(8),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[19~"),
    },
    Case {
        name: "f9 is CSI 20 tilde",
        key: Key::Function(9),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[20~"),
    },
    Case {
        name: "f10 is CSI 21 tilde",
        key: Key::Function(10),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[21~"),
    },
    Case {
        name: "f11 is CSI 23 tilde — 22 is skipped too",
        key: Key::Function(11),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[23~"),
    },
    Case {
        name: "f12 is CSI 24 tilde",
        key: Key::Function(12),
        modifiers: NONE,
        cursor: NORMAL,
        want: Some(b"\x1b[24~"),
    },
    Case {
        name: "there is no f0, so it sends nothing",
        key: Key::Function(0),
        modifiers: NONE,
        cursor: NORMAL,
        want: None,
    },
    Case {
        name: "f13 and up have no sequence here, so they send nothing",
        key: Key::Function(13),
        modifiers: NONE,
        cursor: NORMAL,
        want: None,
    },
];

#[test]
fn corpus_holds() {
    for case in CORPUS {
        let got = encode(case.key, case.modifiers, case.cursor);
        match case.want {
            None => assert_eq!(got, None, "{}: expected no bytes", case.name),
            Some(want) => assert_eq!(
                got.as_deref(),
                Some(want),
                "{}: wrong bytes for the key",
                case.name
            ),
        }
    }
}

/// Every key the table names must appear in the corpus, in both cursor modes
/// for the keys that read the mode. Adding a variant breaks the table's
/// `match` first; this stops the variant landing with no case to pin its bytes.
#[test]
fn corpus_covers_every_key() {
    let keys = [
        Key::Char('a'),
        Key::Enter,
        Key::Backspace,
        Key::Tab,
        Key::Escape,
        Key::Up,
        Key::Down,
        Key::Right,
        Key::Left,
        Key::Home,
        Key::End,
        Key::PageUp,
        Key::PageDown,
        Key::Insert,
        Key::Delete,
        Key::Function(1),
    ];
    for key in keys {
        assert!(
            CORPUS.iter().any(|c| c.key == key),
            "no corpus case for {key:?}"
        );
    }
    for n in 1..=12u8 {
        assert!(
            CORPUS.iter().any(|c| c.key == Key::Function(n)),
            "no corpus case for F{n}"
        );
    }
    for cursor in [CursorKeys::Normal, CursorKeys::Application] {
        assert!(
            CORPUS
                .iter()
                .any(|c| c.cursor == cursor && c.want.is_some()),
            "no corpus case for {cursor:?}"
        );
    }
}

/// The six cursor keys are the whole of what application mode changes. A
/// seventh key quietly reading the mode would break this.
#[test]
fn application_mode_changes_the_cursor_keys_and_nothing_else() {
    let keys = [
        Key::Char('a'),
        Key::Enter,
        Key::Backspace,
        Key::Tab,
        Key::Escape,
        Key::PageUp,
        Key::PageDown,
        Key::Insert,
        Key::Delete,
        Key::Function(1),
        Key::Function(5),
    ];
    for key in keys {
        assert_eq!(
            encode(key, Modifiers::NONE, CursorKeys::Normal),
            encode(key, Modifiers::NONE, CursorKeys::Application),
            "{key:?} must not read the cursor-key mode"
        );
    }
    for key in [
        Key::Up,
        Key::Down,
        Key::Right,
        Key::Left,
        Key::Home,
        Key::End,
    ] {
        assert_ne!(
            encode(key, Modifiers::NONE, CursorKeys::Normal),
            encode(key, Modifiers::NONE, CursorKeys::Application),
            "{key:?} must read the cursor-key mode"
        );
    }
}
