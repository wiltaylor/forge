//! The Forge block key corpus, and the three things every Rust driver of it
//! needs: a typed reading of `contract/blocks/corpus.json`, the id-blind
//! document comparison a case is judged by, and the runner that reports every
//! mismatch at once.
//!
//! Nothing here knows about an editor. A driver supplies the kit — it puts
//! that kit's editor at the case's address in the case's mode, feeds the
//! case's keys, and hands the resulting [`Document`] back to [`run`].
//!
//! ```no_run
//! # use forge_block_corpus::Corpus;
//! let corpus = Corpus::load().unwrap();
//! for case in corpus.cases_for("rust-tui") {
//!     assert!(!case.keys.is_empty());
//! }
//! ```

use std::collections::BTreeMap;

use forge_blocks::{new_id, Address, Block, Document, DOCUMENT_VERSION};
use serde::Deserialize;
use serde_json::Value;

/// The corpus as authored, verbatim.
pub const CORPUS_JSON: &str = include_str!("../../../contract/blocks/corpus.json");

/// Kit id of the ratatui driver.
pub const RUST_TUI: &str = "rust-tui";
/// Kit id of the egui driver.
pub const RUST_EGUI: &str = "rust-egui";

/// One authored block key corpus.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    /// Corpus format version — see `contract/blocks/README.md`.
    pub corpus_version: String,
    /// Every kit a case must account for.
    pub kits: Vec<String>,
    /// The cases, in authored order.
    pub cases: Vec<Case>,
}

/// One editing case: a starting document, an address, a key sequence, and the
/// document that must result.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// Unique, kebab-case.
    pub id: String,
    /// One line, present tense.
    pub title: String,
    /// Optional: why the case is written this way.
    #[serde(default)]
    pub note: Option<String>,
    /// Kits that must produce `expect`.
    pub applies: Vec<String>,
    /// Kits that cannot serve the case, each with a reason.
    #[serde(default)]
    pub inapplicable: BTreeMap<String, String>,
    /// Kits known to produce something *other* than `expect` today, each with
    /// the issue that closes the gap. A driver asserts the difference is still
    /// there, so closing the gap fails the run until the note goes.
    #[serde(default)]
    pub diverges: BTreeMap<String, Divergence>,
    /// Starting document, as blocks without ids.
    pub doc: Vec<Value>,
    /// Where the editor starts.
    pub at: At,
    /// The keys to press, in order.
    pub keys: Vec<Key>,
    /// The document the keys must produce, as blocks without ids.
    pub expect: Vec<Value>,
}

/// A known gap between a kit and the case it fails.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Divergence {
    /// The issue that closes it.
    pub issue: u32,
    /// What the kit does instead.
    pub why: String,
}

/// Where the editor is when the first key arrives.
///
/// `block` indexes the document root. Add `column` + `index` to address a
/// block inside a column cell. Then pick the mode: `caret` for a text caret at
/// that byte offset, `row` + `col` for a table cell (row 0 is the header), or
/// neither for block selection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct At {
    /// Root index of the block — or of the columns container, with `column`.
    pub block: usize,
    /// Column index inside the columns block at `block`.
    #[serde(default)]
    pub column: Option<usize>,
    /// Block index inside that column.
    #[serde(default)]
    pub index: Option<usize>,
    /// Text mode: caret at this byte offset in the block's markdown source.
    #[serde(default)]
    pub caret: Option<usize>,
    /// Table-cell mode: display row, 0 being the header.
    #[serde(default)]
    pub row: Option<usize>,
    /// Table-cell mode: column.
    #[serde(default)]
    pub col: Option<usize>,
}

/// What the editor is doing when the keys arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Block-selected: structural keys, no text caret.
    Select,
    /// Text caret at a byte offset in the block's markdown source.
    Text(usize),
    /// A table cell: (display row, column), row 0 being the header.
    Cell(usize, usize),
}

impl At {
    /// The address this case starts at.
    pub fn address(&self) -> Address {
        match (self.column, self.index) {
            (Some(col), Some(idx)) => Address::Cell {
                root: self.block,
                col,
                idx,
            },
            _ => Address::Root(self.block),
        }
    }

    pub fn mode(&self) -> Mode {
        match (self.caret, self.row, self.col) {
            (_, Some(row), Some(col)) => Mode::Cell(row, col),
            (Some(caret), _, _) => Mode::Text(caret),
            _ => Mode::Select,
        }
    }

    fn validate(&self, id: &str) -> Result<(), String> {
        if self.column.is_some() != self.index.is_some() {
            return Err(format!(
                "{id}: `at` needs both `column` and `index`, or neither"
            ));
        }
        if self.row.is_some() != self.col.is_some() {
            return Err(format!("{id}: `at` needs both `row` and `col`, or neither"));
        }
        if self.caret.is_some() && self.row.is_some() {
            return Err(format!(
                "{id}: `at` is either a text caret or a table cell, not both"
            ));
        }
        Ok(())
    }
}

/// One keypress in the browser `KeyboardEvent` vocabulary this repo already
/// uses for its remote-protocol keymaps (`crates/forge-core/src/widgets/keymap`):
/// a layout-independent `code`, plus the produced character in `key` when the
/// key is printable.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    /// `KeyboardEvent.code` — `"Enter"`, `"Backspace"`, `"KeyA"`, `"Slash"`, …
    pub code: String,
    /// `KeyboardEvent.key` for printables — the character to insert.
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

impl Case {
    /// The starting document. Ids are minted here — the corpus does not author
    /// them, because block identity is not part of the editing policy.
    pub fn document(&self) -> Document {
        document_of(&self.doc).unwrap_or_else(|e| panic!("{}: doc: {e}", self.id))
    }

    /// The document the keys must produce.
    pub fn expected(&self) -> Document {
        document_of(&self.expect).unwrap_or_else(|e| panic!("{}: expect: {e}", self.id))
    }

    /// Every key pressed, for a failure report.
    pub fn keys_label(&self) -> String {
        self.keys
            .iter()
            .map(Key::label)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Read authored, id-less blocks as a document. Ids are minted on the way in
/// so the schema's own deserializer does the shape checking.
fn document_of(blocks: &[Value]) -> Result<Document, String> {
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        let mut block = block.clone();
        mint_ids(&mut block);
        let block: Block = serde_json::from_value(block).map_err(|e| e.to_string())?;
        out.push(block);
    }
    Ok(Document {
        version: DOCUMENT_VERSION,
        blocks: out,
    })
}

/// Give a block — and every block in its column cells — an id.
fn mint_ids(block: &mut Value) {
    let Some(obj) = block.as_object_mut() else {
        return;
    };
    obj.entry("id").or_insert_with(|| Value::String(new_id()));
    let Some(columns) = obj.get_mut("columns").and_then(Value::as_array_mut) else {
        return;
    };
    for column in columns {
        let Some(nested) = column.get_mut("blocks").and_then(Value::as_array_mut) else {
            continue;
        };
        for block in nested {
            mint_ids(block);
        }
    }
}

impl Corpus {
    /// Parse and validate the authored corpus.
    pub fn load() -> Result<Corpus, String> {
        let corpus: Corpus =
            serde_json::from_str(CORPUS_JSON).map_err(|e| format!("corpus.json: {e}"))?;
        corpus.validate()?;
        Ok(corpus)
    }

    /// Every rule a case must keep, so a gap has to be written down rather
    /// than created by forgetting.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for case in &self.cases {
            let id = &case.id;
            if seen.contains(&case.id.as_str()) {
                return Err(format!("{id}: duplicate case id"));
            }
            seen.push(&case.id);
            if case.keys.is_empty() {
                return Err(format!("{id}: a case presses at least one key"));
            }
            if case.doc.is_empty() || case.expect.is_empty() {
                return Err(format!("{id}: a document is never blockless"));
            }
            document_of(&case.doc).map_err(|e| format!("{id}: doc: {e}"))?;
            document_of(&case.expect).map_err(|e| format!("{id}: expect: {e}"))?;
            case.at.validate(id)?;
            for kit in case
                .applies
                .iter()
                .chain(case.inapplicable.keys())
                .chain(case.diverges.keys())
            {
                if !self.kits.contains(kit) {
                    return Err(format!("{id}: unknown kit {kit:?}"));
                }
            }
            for kit in &self.kits {
                let stated = usize::from(case.applies.contains(kit))
                    + usize::from(case.inapplicable.contains_key(kit))
                    + usize::from(case.diverges.contains_key(kit));
                match stated {
                    1 => {}
                    0 => {
                        return Err(format!(
                            "{id}: kit {kit:?} is in neither applies, inapplicable nor diverges"
                        ))
                    }
                    _ => return Err(format!("{id}: kit {kit:?} is stated more than once")),
                }
            }
            for (kit, reason) in &case.inapplicable {
                if reason.trim().is_empty() {
                    return Err(format!("{id}: kit {kit:?} is inapplicable with no reason"));
                }
            }
        }
        Ok(())
    }

    /// The cases `kit` must pass, in authored order.
    pub fn cases_for<'a>(&'a self, kit: &'a str) -> impl Iterator<Item = &'a Case> {
        self.cases
            .iter()
            .filter(move |c| c.applies.iter().any(|k| k == kit))
    }

    /// The cases `kit` is known to fail, with the issue that closes each.
    pub fn divergences_for<'a>(
        &'a self,
        kit: &'a str,
    ) -> impl Iterator<Item = (&'a Case, &'a Divergence)> {
        self.cases
            .iter()
            .filter_map(move |c| c.diverges.get(kit).map(|d| (c, d)))
    }
}

/* ---------------- comparison -------------------------------------------- */

/// A document as the corpus judges it: the wire JSON with every block id
/// removed. Block identity is editor bookkeeping, not editing policy — two
/// documents that differ only by id are the same document to a case.
pub fn judged(doc: &Document) -> Value {
    let mut value = serde_json::to_value(doc).expect("Document serializes");
    if let Some(blocks) = value.get_mut("blocks") {
        strip_ids(blocks);
    }
    value
}

/// Drop `id` from every block in a `blocks` array, through column cells.
fn strip_ids(blocks: &mut Value) {
    let Some(list) = blocks.as_array_mut() else {
        return;
    };
    for block in list {
        let Some(obj) = block.as_object_mut() else {
            continue;
        };
        obj.remove("id");
        let Some(columns) = obj.get_mut("columns").and_then(Value::as_array_mut) else {
            continue;
        };
        for column in columns {
            if let Some(nested) = column.get_mut("blocks") {
                strip_ids(nested);
            }
        }
    }
}

/* ---------------- the runner -------------------------------------------- */

/// Run the whole corpus against one kit and panic with every mismatch at once.
///
/// `drive` puts the kit's editor at `case.at`, presses `case.keys` in order,
/// and returns the document that resulted. It runs for the kit's `applies`
/// cases *and* its `diverges` cases: an applies case must match `expect`, and
/// a diverges case must **not** — so the run turns red the moment a known gap
/// closes and the note is stale.
pub fn run(kit: &str, mut drive: impl FnMut(&Case) -> Document) {
    let corpus = Corpus::load().unwrap_or_else(|e| panic!("{e}"));
    let mut failures: Vec<String> = Vec::new();
    let mut ran = 0usize;

    for case in &corpus.cases {
        let expected_to_match = if case.applies.iter().any(|k| k == kit) {
            true
        } else if case.diverges.contains_key(kit) {
            false
        } else {
            continue;
        };
        ran += 1;
        let actual = judged(&drive(case));
        let expected = judged(&case.expected());
        match (expected_to_match, actual == expected) {
            (true, true) | (false, false) => {}
            (true, false) => failures.push(format!(
                "{id}: {title}\n  keys: {keys}\n  expected: {expected}\n  actual:   {actual}",
                id = case.id,
                title = case.title,
                keys = case.keys_label(),
            )),
            (false, true) => {
                let d = &case.diverges[kit];
                failures.push(format!(
                    "{id}: {title}\n  the corpus records this as a known {kit} divergence \
                     closed by #{issue}, but {kit} now matches it.\n  \
                     Drop the `diverges` note from the case.\n  recorded: {why}",
                    id = case.id,
                    title = case.title,
                    issue = d.issue,
                    why = d.why,
                ));
            }
        }
    }

    assert!(ran > 0, "no corpus case names the kit {kit:?}");
    assert!(
        failures.is_empty(),
        "{n} of {ran} block corpus cases failed against {kit}:\n\n{report}",
        n = failures.len(),
        report = failures.join("\n\n"),
    );
}
