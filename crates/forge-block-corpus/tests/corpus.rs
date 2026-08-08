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

/// The known kit divergences are written down against the kit they belong to,
/// with the issue that closes them. #28 is that issue.
#[test]
fn the_known_divergences_are_recorded() {
    let corpus = corpus();
    let ids = |kit| {
        corpus
            .divergences_for(kit)
            .map(|(case, d)| {
                assert_eq!(d.issue, 28, "{}: {kit} names an unexpected issue", case.id);
                assert!(!d.why.trim().is_empty(), "{}: {kit} has no reason", case.id);
                case.id.as_str()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(RUST_TUI),
        [
            "slash-palette-offers-a-heading-4",
            "slash-palette-starts-a-three-by-two-table",
        ],
        "the ratatui divergences changed — is that intended?"
    );
    assert_eq!(
        ids(RUST_EGUI),
        ["delete-at-the-end-merges-the-next-paragraph-forward"],
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
