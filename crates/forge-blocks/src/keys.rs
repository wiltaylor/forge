//! Key-to-operation resolution: the editing policy every Rust kit shares.
//!
//! A kit measures and paints. It does not decide what a key means. It hands
//! [`resolve_key`] a keypress in the normalised [`Key`] shape, the address of
//! the focused block, what it is doing with that block ([`Mode`]), and the
//! document — and gets back the [`Op`] to perform.
//!
//! `None` is not "unbound": it means the key belongs to the kit's own text
//! buffer and caret, which need the wrapped geometry only the kit has. Every
//! key that reads or writes the *document* resolves here.
//!
//! The demote-before-merge rule lives here: Backspace at offset 0 of a
//! non-paragraph text block resolves to [`Op::Demote`], and only a paragraph
//! resolves to [`Op::Merge`]. That is why [`crate::merge_with_previous`] can
//! merge paragraphs alone — the rule in front of it has one author now.

use crate::address::{flatten_addresses, next_address, Address};
use crate::ops::line_start_shortcut;
use crate::schema::{BlockKind, Document};
use serde::Deserialize;

/// One keypress in the browser `KeyboardEvent` vocabulary this repo already
/// uses for its remote-protocol keymaps (`crates/forge-core/src/widgets/keymap`):
/// a layout-independent `code`, plus the produced character in `key` when the
/// key is printable.
///
/// This is the shape `contract/blocks/corpus.json` authors its keys in, so a
/// kit that adapts its own key type onto it is speaking the corpus's language
/// as well as the resolver's.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    /// `KeyboardEvent.code` — `"Enter"`, `"Backspace"`, `"KeyA"`, `"Slash"`, …
    pub code: String,
    /// `KeyboardEvent.key` for printables — the character the key produces.
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
}

impl Key {
    /// A key named by its `code`, with no modifiers and no character.
    pub fn new(code: impl Into<String>) -> Key {
        Key {
            code: code.into(),
            ..Key::default()
        }
    }

    /// The keypress that types `c`, on the US layout the vocabulary is
    /// written against. A character the layout does not name (an accented
    /// letter, a CJK glyph, an emoji) keeps `"Unidentified"` — the browser's
    /// own name for a key with no `code` — because what the editor does with
    /// a printable is decided by the character, not the physical key.
    pub fn typed(c: char) -> Key {
        let (code, shift) = code_for_char(c).unwrap_or(("Unidentified", false));
        Key {
            code: code.to_string(),
            key: Some(c.to_string()),
            shift,
            ctrl: false,
            alt: false,
        }
    }

    /// Hold Shift as well.
    pub fn shift(mut self) -> Key {
        self.shift = true;
        self
    }

    /// Hold Ctrl as well.
    pub fn ctrl(mut self) -> Key {
        self.ctrl = true;
        self
    }

    /// Hold Alt as well.
    pub fn alt(mut self) -> Key {
        self.alt = true;
        self
    }

    /// The character this key produces, when it produces one.
    pub fn char(&self) -> Option<char> {
        let key = self.key.as_deref()?;
        let mut chars = key.chars();
        let c = chars.next()?;
        chars.next().is_none().then_some(c)
    }

    /// How the key reads in a failure report: `Shift+Tab`, `KeyA "a"`.
    pub fn label(&self) -> String {
        let mut out = String::new();
        for (on, name) in [
            (self.ctrl, "Ctrl+"),
            (self.alt, "Alt+"),
            (self.shift, "Shift+"),
        ] {
            if on {
                out.push_str(name);
            }
        }
        out.push_str(&self.code);
        if let Some(key) = &self.key {
            out.push_str(&format!(" {key:?}"));
        }
        out
    }
}

/// `KeyboardEvent.code` of the US-layout key that types `c`, and whether it
/// needs Shift. The inverse of the layout tables the remote-protocol keymaps
/// read, for a kit whose key type reports the character but not the key.
fn code_for_char(c: char) -> Option<(&'static str, bool)> {
    const LETTERS: [&str; 26] = [
        "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ", "KeyK",
        "KeyL", "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT", "KeyU", "KeyV",
        "KeyW", "KeyX", "KeyY", "KeyZ",
    ];
    const DIGITS: [&str; 10] = [
        "Digit0", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8",
        "Digit9",
    ];
    if c.is_ascii_lowercase() {
        return Some((LETTERS[c as usize - 'a' as usize], false));
    }
    if c.is_ascii_uppercase() {
        return Some((LETTERS[c as usize - 'A' as usize], true));
    }
    if c.is_ascii_digit() {
        return Some((DIGITS[c as usize - '0' as usize], false));
    }
    let pair = match c {
        ')' => ("Digit0", true),
        '!' => ("Digit1", true),
        '@' => ("Digit2", true),
        '#' => ("Digit3", true),
        '$' => ("Digit4", true),
        '%' => ("Digit5", true),
        '^' => ("Digit6", true),
        '&' => ("Digit7", true),
        '*' => ("Digit8", true),
        '(' => ("Digit9", true),
        ' ' => ("Space", false),
        '-' => ("Minus", false),
        '_' => ("Minus", true),
        '=' => ("Equal", false),
        '+' => ("Equal", true),
        '[' => ("BracketLeft", false),
        '{' => ("BracketLeft", true),
        ']' => ("BracketRight", false),
        '}' => ("BracketRight", true),
        '\\' => ("Backslash", false),
        '|' => ("Backslash", true),
        ';' => ("Semicolon", false),
        ':' => ("Semicolon", true),
        '\'' => ("Quote", false),
        '"' => ("Quote", true),
        '`' => ("Backquote", false),
        '~' => ("Backquote", true),
        ',' => ("Comma", false),
        '<' => ("Comma", true),
        '.' => ("Period", false),
        '>' => ("Period", true),
        '/' => ("Slash", false),
        '?' => ("Slash", true),
        _ => return None,
    };
    Some(pair)
}

/// What the editor is doing with the block at the address when a key arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Block-selected: structural keys, no text caret.
    Select,
    /// A text caret at a byte offset into the block's markdown source.
    Text { caret: usize },
    /// One table cell: display row (0 is the header) and column.
    Cell { row: usize, col: usize },
    /// A buffer of the kit's own holds the block's content — a code body, a
    /// data block's JSON source. The block-level keys still apply; every
    /// other key is the buffer's.
    Buffer,
}

/// What a key means, given the document, the address and the focus.
///
/// An op names the edit, not the bookkeeping: the kit performs it through
/// [`crate::ops`] and re-seats its own caret, scroll and popups afterwards.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// The key is bound here but has nothing to do — the editor consumes it
    /// and the document is unchanged (Delete at the end of the last block,
    /// a selection step off the end).
    Nothing,
    /// Type one character into the focused buffer: a printable, or the soft
    /// line break Shift+Enter and Alt+Enter make.
    Insert(char),
    /// Split the focused text block at `caret`.
    Split { caret: usize },
    /// Turn the text block at `addr` into a paragraph, keeping its markdown.
    /// The rule that stands between Backspace-at-0 and a merge.
    Demote { addr: Address },
    /// Append the paragraph at `addr` to the block above it.
    Merge { addr: Address },
    /// Replace the focused block's kind and put the caret at `caret` — a
    /// line-start markdown shortcut fired as the character was typed, so the
    /// character never reaches the buffer.
    Convert { kind: BlockKind, caret: usize },
    /// Indent (+1) or outdent (-1) the focused list item.
    Indent { delta: i8 },
    /// Swap the focused block with its sibling in `dir` (-1 up, +1 down).
    MoveBlock { dir: i32 },
    /// Remove the focused block.
    Remove,
    /// Cycle the focused admonition's tone.
    CycleTone,
    /// Wrap the focused block into an `n`-column layout.
    WrapColumns { n: usize },
    /// Select `addr` in block mode — either a step of the selection, or
    /// dropping out of text/cell editing onto the block being edited.
    Select { addr: Address },
    /// Leave the editor: nothing is focused.
    Blur,
    /// Enter the focused block for editing, caret at its end.
    Enter,
    /// Open the block palette on the focused block.
    OpenPalette,
    /// Put the table caret in another cell (display row 0 is the header).
    FocusCell { row: usize, col: usize },
    /// Insert an empty table row at `at`, then focus the cell in `focus`.
    InsertRow {
        at: usize,
        focus: Option<(usize, usize)>,
    },
    /// Insert an empty table column at `at`.
    InsertCol { at: usize },
    /// Remove the table column at `at`.
    RemoveCol { at: usize },
}

/// Resolve a keypress against the document into the operation it means.
///
/// `None` hands the key back to the kit: caret movement, buffer editing, or a
/// key the editor does not bind. Every op that touches the document comes out
/// of here, so two kits cannot answer the same keypress differently.
pub fn resolve_key(doc: &Document, addr: Address, mode: Mode, key: &Key) -> Option<Op> {
    match mode {
        Mode::Text { caret } => text_key(doc, addr, caret, key),
        Mode::Cell { row, col } => cell_key(doc, addr, row, col, key),
        Mode::Select => select_key(doc, addr, key),
        Mode::Buffer => buffer_key(addr, key),
    }
}

/// The block-level keys a kit's own buffer leaves through: leave the buffer,
/// and move the block it belongs to.
fn buffer_key(addr: Address, key: &Key) -> Option<Op> {
    match key.code.as_str() {
        "Escape" => Some(Op::Select { addr }),
        "ArrowUp" if key.alt => Some(Op::MoveBlock { dir: -1 }),
        "ArrowDown" if key.alt => Some(Op::MoveBlock { dir: 1 }),
        _ => None,
    }
}

/* ---------------- text caret -------------------------------------------- */

/// Keys with a text caret in the block's markdown source. Everything the
/// caret itself does — moving it, deleting either side of it — is the kit's,
/// because only the kit knows where the text wrapped.
fn text_key(doc: &Document, addr: Address, caret: usize, key: &Key) -> Option<Op> {
    // Escape leaves the text whatever the block turns out to hold.
    if key.code == "Escape" {
        return Some(Op::Select { addr });
    }
    let kind = &doc.block(addr)?.kind;
    let md = kind.md()?;
    let caret = clamp(md, caret);
    match key.code.as_str() {
        "Enter" | "NumpadEnter" if key.shift || key.alt => Some(Op::Insert('\n')),
        "Enter" | "NumpadEnter" => Some(Op::Split { caret }),
        // Backspace inside the text is the kit's; at offset 0 it is a merge —
        // and a non-paragraph text block demotes first, one Backspace per step.
        "Backspace" if caret > 0 => None,
        "Backspace" => Some(match kind {
            BlockKind::Paragraph { .. } => Op::Merge { addr },
            _ => Op::Demote { addr },
        }),
        // Delete at the end is Backspace-at-0 of the block below, so it is the
        // same merge one address on. Merging a non-paragraph is refused there.
        "Delete" if caret < md.len() => None,
        "Delete" => Some(match next_address(doc, addr) {
            Some(next) => Op::Merge { addr: next },
            None => Op::Nothing,
        }),
        "Tab" if matches!(kind, BlockKind::ListItem { .. }) => Some(Op::Indent {
            delta: if key.shift { -1 } else { 1 },
        }),
        "ArrowUp" if key.alt => Some(Op::MoveBlock { dir: -1 }),
        "ArrowDown" if key.alt => Some(Op::MoveBlock { dir: 1 }),
        _ => match key.char()? {
            't' if key.ctrl => Some(Op::CycleTone),
            _ if key.ctrl || key.alt => None,
            '/' if md.is_empty() => Some(Op::OpenPalette),
            c => Some(convert(kind, md, caret, c).unwrap_or(Op::Insert(c))),
        },
    }
}

/// The line-start shortcut the block would read as, were `typed` inserted at
/// `caret`. Only a paragraph converts, and the caret pulls back over the
/// prefix the shortcut consumed.
fn convert(kind: &BlockKind, md: &str, caret: usize, typed: char) -> Option<Op> {
    if !matches!(kind, BlockKind::Paragraph { .. }) {
        return None;
    }
    let mut next = String::with_capacity(md.len() + typed.len_utf8());
    next.push_str(&md[..caret]);
    next.push(typed);
    next.push_str(&md[caret..]);
    let shortcut = line_start_shortcut(&next)?;
    Some(Op::Convert {
        kind: shortcut.kind,
        caret: (caret + typed.len_utf8()).saturating_sub(shortcut.prefix_len),
    })
}

/// A byte offset inside `s`, on a character boundary.
fn clamp(s: &str, byte: usize) -> usize {
    let mut b = byte.min(s.len());
    while b > 0 && !s.is_char_boundary(b) {
        b -= 1;
    }
    b
}

/* ---------------- table cell -------------------------------------------- */

/// Keys with the caret in one table cell. Walking the cells and growing the
/// table are the table's business; the text inside a cell is the kit's.
fn cell_key(doc: &Document, addr: Address, row: usize, col: usize, key: &Key) -> Option<Op> {
    // Escape leaves the cell whatever the block turns out to be.
    if key.code == "Escape" {
        return Some(Op::Select { addr });
    }
    let BlockKind::Table { header, rows } = &doc.block(addr)?.kind else {
        return None;
    };
    // Display rows are the header plus the body, so the last row index is
    // `rows.len()` and a table with no body rows is still one row deep.
    let (ncols, nrows) = (header.len().max(1), rows.len());
    match key.code.as_str() {
        "Tab" if key.shift => Some(if col > 0 {
            Op::FocusCell { row, col: col - 1 }
        } else if row > 0 {
            Op::FocusCell {
                row: row - 1,
                col: ncols - 1,
            }
        } else {
            Op::FocusCell {
                row: nrows,
                col: ncols - 1,
            }
        }),
        "Tab" => Some(if col + 1 < ncols {
            Op::FocusCell { row, col: col + 1 }
        } else if row < nrows {
            Op::FocusCell {
                row: row + 1,
                col: 0,
            }
        } else {
            Op::FocusCell { row: 0, col: 0 }
        }),
        "ArrowUp" if !key.alt => Some(Op::FocusCell {
            row: row.saturating_sub(1),
            col,
        }),
        "ArrowDown" if !key.alt => Some(Op::FocusCell {
            row: (row + 1).min(nrows),
            col,
        }),
        // Ctrl+Enter inserts under the focused row and stays put; plain Enter
        // steps down, growing the table only off the last row.
        "Enter" | "NumpadEnter" if key.ctrl => Some(Op::InsertRow {
            at: if row == 0 { 0 } else { row },
            focus: None,
        }),
        "Enter" | "NumpadEnter" if row < nrows => Some(Op::FocusCell { row: row + 1, col }),
        "Enter" | "NumpadEnter" => Some(Op::InsertRow {
            at: nrows,
            focus: Some((row + 1, col)),
        }),
        _ => match key.char()? {
            '=' if key.alt => Some(Op::InsertCol { at: col + 1 }),
            '-' if key.alt => Some(Op::RemoveCol { at: col }),
            _ if key.ctrl || key.alt => None,
            c => Some(Op::Insert(c)),
        },
    }
}

/* ---------------- block selection --------------------------------------- */

/// Keys with the block selected and no caret anywhere: the structural ones.
fn select_key(doc: &Document, addr: Address, key: &Key) -> Option<Op> {
    match key.code.as_str() {
        "ArrowUp" if key.alt => Some(Op::MoveBlock { dir: -1 }),
        "ArrowDown" if key.alt => Some(Op::MoveBlock { dir: 1 }),
        "ArrowUp" => Some(step(doc, addr, -1)),
        "ArrowDown" => Some(step(doc, addr, 1)),
        "Enter" | "NumpadEnter" => Some(Op::Enter),
        "Delete" | "Backspace" => Some(Op::Remove),
        "Escape" => Some(match addr {
            Address::Cell { root, .. } => Op::Select {
                addr: Address::Root(root),
            },
            Address::Root(_) => Op::Blur,
        }),
        _ => match key.char()? {
            '/' => Some(Op::OpenPalette),
            'c' if !key.ctrl && !key.alt => Some(Op::WrapColumns { n: 2 }),
            't' if key.ctrl => Some(Op::CycleTone),
            _ => None,
        },
    }
}

/// Where the block selection lands after a step of `dir` in navigation order.
/// A `Columns` container is not a navigation stop, so a selection sitting on
/// one steps to the nearest block outside it.
fn step(doc: &Document, addr: Address, dir: i32) -> Op {
    let flat = flatten_addresses(doc);
    let landing = match flat.iter().position(|a| *a == addr) {
        Some(pos) => pos
            .checked_add_signed(dir as isize)
            .and_then(|next| flat.get(next)),
        None => {
            let root = addr.root();
            let pos = if dir < 0 {
                flat.iter().rposition(|a| a.root() < root)
            } else {
                flat.iter().position(|a| a.root() > root)
            };
            pos.and_then(|p| flat.get(p))
        }
    };
    match landing {
        Some(addr) => Op::Select { addr: *addr },
        None => Op::Nothing,
    }
}
