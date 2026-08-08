//! Pure document operations.
//!
//! What a keypress does with these is the block key corpus's, not this file's.
//! The corpus is `contract/blocks/corpus.json`. Three drivers run it:
//! `crates/forge-tui/tests/block_corpus.rs`, `crates/forge-egui/tests/block_corpus.rs`
//! and `packages/blocks/tests/block_corpus.test.tsx`. It states splitting,
//! merging, the demote-before-merge rule, indent clamping, block moves, the
//! line-start shortcut grammar and the table keys. Two languages used to assert
//! that model by hand, in near-duplicate suites; the corpus replaced them, so a
//! rule is authored once and covers both.
//!
//! What stays here is what the corpus does not state: block removal, block
//! moves at the end of a list, column wrapping and ratios, table row removal,
//! the shortcut spellings no case types, and the markdown conversion.

use forge_blocks::*;

fn p(md: &str) -> Block {
    Block::new(BlockKind::Paragraph { md: md.into() })
}

fn doc(blocks: Vec<Block>) -> Document {
    Document::from_blocks(blocks)
}

fn md_at(d: &Document, addr: Address) -> &str {
    d.block(addr).unwrap().kind.md().unwrap()
}

#[test]
fn remove_refocuses_and_refills() {
    let mut d = doc(vec![p("a"), p("b"), p("c")]);
    let focus = remove(&mut d, Address::Root(1)).unwrap();
    assert_eq!(focus, Address::Root(0));
    assert_eq!(d.blocks.len(), 2);

    // Removing the last remaining block leaves one empty paragraph.
    let mut d = doc(vec![p("only")]);
    let focus = remove(&mut d, Address::Root(0)).unwrap();
    assert_eq!(focus, Address::Root(0));
    assert_eq!(md_at(&d, Address::Root(0)), "");
}

/// The bottom edge of a move. The corpus states the top one
/// (`alt-up-at-the-top-changes-nothing`) and the move itself
/// (`alt-down-moves-a-block-past-its-next-sibling`), but no case presses Alt+Down
/// on the last block.
#[test]
fn move_stops_at_the_last_sibling() {
    let mut d = doc(vec![p("a"), p("b")]);
    assert!(move_block(&mut d, Address::Root(1), 1).is_none());
}

#[test]
fn columns_wrap_navigate_unwrap() {
    let mut d = doc(vec![p("a"), p("b")]);
    let focus = wrap_in_columns(&mut d, Address::Root(0), 2).unwrap();
    assert_eq!(
        focus,
        Address::Cell {
            root: 0,
            col: 0,
            idx: 0
        }
    );
    assert_eq!(md_at(&d, focus), "a");

    // Navigation flattens through the columns.
    let flat = flatten_addresses(&d);
    assert_eq!(flat.len(), 3); // cell a, empty cell paragraph, root b
    assert_eq!(next_address(&d, focus).unwrap(), flat[1]);

    // No nested columns.
    assert!(wrap_in_columns(&mut d, focus, 2).is_none());
    assert!(!set_kind(
        &mut d,
        focus,
        BlockKind::Columns { columns: vec![] }
    ));

    // Add a third column, then remove it again.
    assert_eq!(add_column(&mut d, 0), Some(2));
    remove_column(&mut d, 0, 2).unwrap();

    // Removing one of two columns unwraps to the root.
    let focus = remove_column(&mut d, 0, 1).unwrap();
    assert_eq!(focus, Address::Root(0));
    assert!(matches!(d.blocks[0].kind, BlockKind::Paragraph { .. }));
    assert_eq!(md_at(&d, Address::Root(0)), "a");
}

#[test]
fn column_ratios_normalize() {
    let mut d = doc(vec![p("a")]);
    wrap_in_columns(&mut d, Address::Root(0), 2).unwrap();
    assert!(set_column_ratios(&mut d, 0, &[3.0, 1.0]));
    match &d.blocks[0].kind {
        BlockKind::Columns { columns } => {
            assert!((columns[0].ratio - 0.75).abs() < 1e-6);
            assert!((columns[1].ratio - 0.25).abs() < 1e-6);
        }
        _ => unreachable!(),
    }
    assert!(!set_column_ratios(&mut d, 0, &[1.0]));
    assert!(!set_column_ratios(&mut d, 0, &[-1.0, 2.0]));
}

/// Row removal, which no key in any kit reaches — the kits remove a row from a
/// menu or a toolbar button. Column removal and both inserts are the corpus's
/// (`table-alt-minus-*`, `table-alt-equals-*`, `table-ctrl-enter-*`).
#[test]
fn table_row_removal_keeps_the_last_row() {
    let mut d = doc(vec![Block::new(BlockKind::Table {
        header: vec!["A".into(), "B".into()],
        rows: vec![vec!["1".into(), "2".into()]],
    })]);
    let addr = Address::Root(0);
    assert!(table_insert_row(&mut d, addr, 1));
    assert!(table_remove_row(&mut d, addr, 1));
    assert!(!table_remove_row(&mut d, addr, 0));
}

/// The spellings no corpus case types. The corpus states the grammar a user
/// reaches by typing at the start of a paragraph (`shortcut-*`); these are the
/// arms it leaves out — the two todo prefixes a dash converts before, a code
/// fence carrying a language, and the spellings that must *not* convert at all.
#[test]
fn shortcuts_the_corpus_does_not_type() {
    assert!(matches!(
        line_start_shortcut("- [x] done").unwrap().kind,
        BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(true),
            ..
        }
    ));
    assert!(matches!(
        line_start_shortcut("- [ ] todo").unwrap().kind,
        BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(false),
            ..
        }
    ));
    assert!(matches!(
        line_start_shortcut("```rust").unwrap().kind,
        BlockKind::Code { ref lang, .. } if lang == "rust"
    ));

    assert!(line_start_shortcut("#x").is_none()); // needs the space
    assert!(line_start_shortcut("-x").is_none());
    assert!(line_start_shortcut("##### five").is_none()); // four levels only
}

#[test]
fn markdown_roundtrip() {
    let d = sample::sample_document();
    let text = to_markdown(&d);
    let back = from_markdown(&text);

    // Columns flatten (documented lossy) — everything else round-trips by kind.
    let flatten = |doc: &Document| -> Vec<String> {
        let mut kinds = Vec::new();
        for b in &doc.blocks {
            match &b.kind {
                BlockKind::Columns { columns } => {
                    for c in columns {
                        for cb in &c.blocks {
                            kinds.push(format!("{:?}", std::mem::discriminant(&cb.kind)));
                        }
                    }
                }
                k => kinds.push(format!("{:?}", std::mem::discriminant(k))),
            }
        }
        kinds
    };
    assert_eq!(flatten(&d), flatten(&back));

    // Custom block data survives the fence.
    let custom = back
        .blocks
        .iter()
        .find_map(|b| match &b.kind {
            BlockKind::Custom { kind, data } => Some((kind.clone(), data.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(custom.0, "counter");
    assert_eq!(custom.1["count"], 3);

    // Admonition tag round-trips tone + title.
    let adm = back
        .blocks
        .iter()
        .find_map(|b| match &b.kind {
            BlockKind::Admonition { tone, title, .. } => Some((*tone, title.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(adm.0, Tone::Warning);
    assert_eq!(adm.1, "Careful");
}

#[test]
fn data_block_markdown_forms() {
    // Data kinds travel as ```forge:<type> fences and round-trip field-for-field.
    let d = doc(vec![
        Block::new(starter_kind("bar_chart").unwrap()),
        Block::new(starter_kind("diagram").unwrap()),
        Block::new(starter_kind("timeline").unwrap()),
    ]);
    let text = to_markdown(&d);
    assert!(text.contains("```forge:bar_chart"));
    let back = from_markdown(&text);
    for (a, b) in d.blocks.iter().zip(back.blocks.iter()) {
        assert_eq!(a.kind, b.kind);
    }

    // Natural-markdown forms.
    let back = from_markdown("![alt text](pic.png)\n\n$$\ne = mc^2\n$$\n\n[^n1]: the note\n");
    assert!(
        matches!(&back.blocks[0].kind, BlockKind::Image { src, alt, width: None, height: None }
            if src == "pic.png" && alt == "alt text")
    );
    assert!(matches!(&back.blocks[1].kind, BlockKind::Math { tex } if tex == "e = mc^2"));
    assert!(
        matches!(&back.blocks[2].kind, BlockKind::Footnote { label, md }
            if label == "n1" && md == "the note")
    );

    // An image with explicit dimensions must NOT flatten to ![alt](src).
    let d = doc(vec![Block::new(BlockKind::Image {
        src: "a.png".into(),
        alt: "A".into(),
        width: Some(640.0),
        height: None,
    })]);
    let text = to_markdown(&d);
    assert!(text.contains("```forge:image"));
    assert_eq!(from_markdown(&text).blocks[0].kind, d.blocks[0].kind);

    // Malformed fence payloads degrade to a code block — content is never lost.
    let back = from_markdown("```forge:bar_chart\n{ not json\n```\n");
    assert!(
        matches!(&back.blocks[0].kind, BlockKind::Code { lang, code }
            if lang == "forge:bar_chart" && code.contains("not json"))
    );
}
