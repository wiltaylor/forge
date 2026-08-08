//! The corpus reads, and the rules that stop a gap being created by
//! forgetting are enforced.

use forge_block_corpus::{Corpus, Mode, RUST_EGUI, RUST_TUI};
use forge_blocks::{Address, BlockKind};

fn corpus() -> Corpus {
    Corpus::load().expect("contract/blocks/corpus.json loads and validates")
}

#[test]
fn the_authored_corpus_loads_and_validates() {
    let corpus = corpus();
    assert_eq!(corpus.corpus_version, "1.0");
    assert!(corpus.cases.len() > 40, "the corpus is not a stub");
    assert!(corpus.kits.iter().any(|k| k == RUST_TUI));
    assert!(corpus.kits.iter().any(|k| k == RUST_EGUI));
}

#[test]
fn every_kit_has_cases_to_run() {
    let corpus = corpus();
    for kit in &corpus.kits {
        assert!(
            corpus.cases_for(kit).next().is_some(),
            "no case applies to {kit}"
        );
    }
}

/// The divergence register is empty: #28 closed all three recorded cases, and
/// each one's kit moved into `applies`.
///
/// A new entry here is not forbidden — it is how a fresh divergence gets
/// written down — but it must name the issue that closes it, and this test is
/// where that gets noticed.
#[test]
fn the_known_divergences_are_recorded() {
    let corpus = corpus();
    let ids = |kit| {
        corpus
            .divergences_for(kit)
            .map(|(case, d)| {
                assert!(d.issue > 0, "{}: {kit} names no closing issue", case.id);
                assert!(!d.why.trim().is_empty(), "{}: {kit} has no reason", case.id);
                case.id.as_str()
            })
            .collect::<Vec<_>>()
    };
    let empty: [&str; 0] = [];
    assert_eq!(
        ids(RUST_TUI),
        empty,
        "the ratatui divergences changed — is that intended?"
    );
    assert_eq!(
        ids(RUST_EGUI),
        empty,
        "the egui divergences changed — is that intended?"
    );
    assert!(ids("web").is_empty(), "the web kit diverges from nothing");
}

#[test]
fn a_case_addresses_a_block_and_a_mode() {
    let corpus = corpus();
    let case = corpus
        .cases
        .iter()
        .find(|c| c.id == "backspace-at-the-start-of-a-column-cell-stays-inside-the-cell")
        .expect("case present");
    assert_eq!(
        case.at.address(),
        Address::Cell {
            root: 0,
            col: 1,
            idx: 0
        }
    );
    assert_eq!(case.at.mode(), Mode::Text(0));

    let table = corpus
        .cases
        .iter()
        .find(|c| c.id == "table-enter-on-the-last-row-appends-a-row")
        .expect("case present");
    assert_eq!(table.at.address(), Address::Root(0));
    assert_eq!(table.at.mode(), Mode::Cell(1, 0));
}

#[test]
fn a_case_builds_typed_documents() {
    let corpus = corpus();
    let case = corpus
        .cases
        .iter()
        .find(|c| c.id == "backspace-demotes-a-heading-before-it-merges")
        .expect("case present");
    assert!(matches!(
        case.document().blocks[1].kind,
        BlockKind::Heading { level: 2, .. }
    ));
    assert!(matches!(
        case.expected().blocks[1].kind,
        BlockKind::Paragraph { .. }
    ));
}

/// The judged form drops block ids, so two documents that differ only by
/// identity are the same document to a case.
#[test]
fn judging_ignores_block_ids() {
    use forge_blocks::{Block, Column, Document};
    let build = || {
        Document::from_blocks(vec![
            Block::new(BlockKind::Paragraph { md: "a".into() }),
            Block::new(BlockKind::Columns {
                columns: vec![Column {
                    ratio: 1.0,
                    blocks: vec![Block::new(BlockKind::Paragraph { md: "b".into() })],
                }],
            }),
        ])
    };
    let (one, two) = (build(), build());
    assert_ne!(one, two, "fresh ids differ");
    assert_eq!(
        forge_block_corpus::judged(&one),
        forge_block_corpus::judged(&two)
    );
}

#[test]
fn a_key_reads_back_in_the_browser_vocabulary() {
    let corpus = corpus();
    let case = corpus
        .cases
        .iter()
        .find(|c| c.id == "shift-tab-outdents-a-list-item")
        .expect("case present");
    assert_eq!(case.keys[0].code, "Tab");
    assert!(case.keys[0].shift);
    assert_eq!(case.keys[0].char(), None);
    assert_eq!(case.keys_label(), "Shift+Tab");
}

/// Every printable the corpus types names the code `Key::typed` gives it.
///
/// A kit whose key type reports the character but not the physical key builds
/// its `Key` with `Key::typed`. That table and the codes authored here are two
/// statements of one layout, and this is what stops them drifting apart.
#[test]
fn the_authored_codes_agree_with_the_shared_layout_table() {
    let corpus = corpus();
    let mut checked = 0;
    for case in &corpus.cases {
        for key in &case.keys {
            let Some(c) = key.char() else { continue };
            let typed = forge_blocks::Key::typed(c);
            assert_eq!(
                typed.code,
                key.code,
                "{}: the corpus types {c:?} as {code:?}, Key::typed as {typed:?}",
                case.id,
                code = key.code,
                typed = typed.code,
            );
            assert_eq!(typed.shift, key.shift, "{}: {c:?} shift", case.id);
            checked += 1;
        }
    }
    assert!(checked > 0, "no corpus case types a character");
}

/* ---------------- the rules, against corpora authored to break them ------ */

/// A corpus of one case, with `patch` merged over a case that would otherwise
/// validate. `null` in the patch removes the key.
fn broken(patch: serde_json::Value) -> String {
    let mut case = serde_json::json!({
        "id": "a-case",
        "title": "Backspace merges",
        "applies": ["rust-tui", "rust-egui", "web"],
        "doc": [{ "type": "paragraph", "md": "a" }, { "type": "paragraph", "md": "b" }],
        "at": { "block": 1, "caret": 0 },
        "keys": [{ "code": "Backspace" }],
        "expect": [{ "type": "paragraph", "md": "ab" }]
    });
    let object = case.as_object_mut().expect("a case is an object");
    for (key, value) in patch.as_object().expect("a patch is an object") {
        if value.is_null() {
            object.remove(key);
        } else {
            object.insert(key.clone(), value.clone());
        }
    }
    serde_json::json!({
        "corpus_version": "1.0",
        "kits": ["rust-tui", "rust-egui", "web"],
        "cases": [case]
    })
    .to_string()
}

fn rejection(patch: serde_json::Value) -> String {
    Corpus::parse(&broken(patch)).expect_err("this corpus must be rejected")
}

#[test]
fn a_case_that_validates_is_the_baseline() {
    Corpus::parse(&broken(serde_json::json!({}))).expect("the unpatched case validates");
}

#[test]
fn a_case_that_ignores_a_kit_is_rejected() {
    let err = rejection(serde_json::json!({ "applies": ["rust-tui", "rust-egui"] }));
    assert!(err.contains("\"web\""), "{err}");
    assert!(err.contains("neither applies"), "{err}");
}

#[test]
fn a_case_that_states_a_kit_twice_is_rejected() {
    let err = rejection(serde_json::json!({
        "inapplicable": { "rust-tui": "a reason" }
    }));
    assert!(err.contains("stated more than once"), "{err}");
}

#[test]
fn an_unknown_kit_is_rejected() {
    let err = rejection(serde_json::json!({
        "applies": ["rust-tui", "rust-egui", "web", "rust-gtk"]
    }));
    assert!(err.contains("unknown kit"), "{err}");
}

/// Both ways of writing a gap down need the reason that makes it reviewable.
#[test]
fn a_gap_without_a_reason_is_rejected() {
    let err = rejection(serde_json::json!({
        "applies": ["rust-egui", "web"],
        "inapplicable": { "rust-tui": "   " }
    }));
    assert!(err.contains("inapplicable with no reason"), "{err}");

    let err = rejection(serde_json::json!({
        "applies": ["rust-egui", "web"],
        "diverges": { "rust-tui": { "issue": 28, "why": "" } }
    }));
    assert!(err.contains("diverges with no reason"), "{err}");
}

#[test]
fn a_case_that_presses_nothing_is_rejected() {
    let err = rejection(serde_json::json!({ "keys": [] }));
    assert!(err.contains("presses at least one key"), "{err}");
}

#[test]
fn a_blockless_document_is_rejected() {
    let err = rejection(serde_json::json!({ "expect": [] }));
    assert!(err.contains("never blockless"), "{err}");
}

#[test]
fn a_block_the_schema_does_not_know_is_rejected() {
    let err = rejection(serde_json::json!({ "doc": [{ "type": "sonnet", "md": "a" }] }));
    assert!(err.contains("doc:"), "{err}");
}

#[test]
fn a_half_written_address_is_rejected() {
    let err = rejection(serde_json::json!({ "at": { "block": 0, "column": 1 } }));
    assert!(err.contains("`column` and `index`"), "{err}");

    let err = rejection(serde_json::json!({ "at": { "block": 0, "row": 1 } }));
    assert!(err.contains("`row` and `col`"), "{err}");

    let err = rejection(serde_json::json!({
        "at": { "block": 0, "caret": 0, "row": 1, "col": 0 }
    }));
    assert!(err.contains("not both"), "{err}");
}

#[test]
fn a_duplicate_case_id_is_rejected() {
    let one = broken(serde_json::json!({}));
    let mut corpus: serde_json::Value = serde_json::from_str(&one).expect("valid json");
    let case = corpus["cases"][0].clone();
    corpus["cases"].as_array_mut().expect("an array").push(case);
    let err = Corpus::parse(&corpus.to_string()).expect_err("duplicate ids are rejected");
    assert!(err.contains("duplicate case id"), "{err}");
}

#[test]
fn an_unknown_field_is_rejected() {
    let err = rejection(serde_json::json!({ "expects": [] }));
    assert!(err.contains("unknown field"), "{err}");
}
