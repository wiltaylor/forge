//! Editing-op behavior shared by every platform's keyboard model.

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
fn split_paragraph() {
    let mut d = doc(vec![p("hello world")]);
    let focus = split(&mut d, Address::Root(0), 5).unwrap();
    assert_eq!(focus, Address::Root(1));
    assert_eq!(md_at(&d, Address::Root(0)), "hello");
    assert_eq!(md_at(&d, Address::Root(1)), " world");
}

#[test]
fn split_heading_tail_is_paragraph() {
    let mut d = doc(vec![Block::new(BlockKind::Heading {
        level: 2,
        md: "ab".into(),
    })]);
    split(&mut d, Address::Root(0), 1).unwrap();
    assert!(matches!(d.blocks[0].kind, BlockKind::Heading { .. }));
    assert!(matches!(d.blocks[1].kind, BlockKind::Paragraph { .. }));
}

#[test]
fn split_list_continues_list_and_empty_converts() {
    let mut d = doc(vec![Block::new(BlockKind::ListItem {
        style: ListStyle::Todo,
        checked: Some(true),
        indent: 2,
        md: "task".into(),
    })]);
    split(&mut d, Address::Root(0), 4).unwrap();
    match &d.blocks[1].kind {
        BlockKind::ListItem {
            style,
            checked,
            indent,
            md,
        } => {
            assert_eq!(*style, ListStyle::Todo);
            assert_eq!(*checked, Some(false)); // new todo starts unchecked
            assert_eq!(*indent, 2);
            assert!(md.is_empty());
        }
        k => panic!("expected list item, got {k:?}"),
    }
    // Enter on the now-empty item converts it to a paragraph in place.
    let focus = split(&mut d, Address::Root(1), 0).unwrap();
    assert_eq!(focus, Address::Root(1));
    assert!(matches!(d.blocks[1].kind, BlockKind::Paragraph { .. }));
    assert_eq!(d.blocks.len(), 2);
}

#[test]
fn merge_appends_and_returns_caret() {
    let mut d = doc(vec![p("ab"), p("cd")]);
    let r = merge_with_previous(&mut d, Address::Root(1)).unwrap();
    assert_eq!(r.focus, Address::Root(0));
    assert_eq!(r.caret, 2);
    assert_eq!(md_at(&d, Address::Root(0)), "abcd");
    assert_eq!(d.blocks.len(), 1);
}

#[test]
fn merge_first_block_is_none_and_divider_deletes() {
    let mut d = doc(vec![p("a")]);
    assert!(merge_with_previous(&mut d, Address::Root(0)).is_none());

    let mut d = doc(vec![p("a"), Block::new(BlockKind::Divider), p("b")]);
    let r = merge_with_previous(&mut d, Address::Root(2)).unwrap();
    assert_eq!(r.focus, Address::Root(1));
    assert_eq!(r.caret, 0);
    assert_eq!(d.blocks.len(), 2);
    assert_eq!(md_at(&d, Address::Root(1)), "b");
}

#[test]
fn move_and_remove() {
    let mut d = doc(vec![p("a"), p("b"), p("c")]);
    let addr = move_block(&mut d, Address::Root(0), 1).unwrap();
    assert_eq!(addr, Address::Root(1));
    assert_eq!(md_at(&d, Address::Root(0)), "b");
    assert!(move_block(&mut d, Address::Root(2), 1).is_none());

    let focus = remove(&mut d, Address::Root(1)).unwrap();
    assert_eq!(focus, Address::Root(0));
    assert_eq!(d.blocks.len(), 2);

    // Removing the last remaining block leaves one empty paragraph.
    let mut d = doc(vec![p("only")]);
    let focus = remove(&mut d, Address::Root(0)).unwrap();
    assert_eq!(focus, Address::Root(0));
    assert_eq!(md_at(&d, Address::Root(0)), "");
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

#[test]
fn table_ops() {
    let mut d = doc(vec![Block::new(BlockKind::Table {
        header: vec!["A".into(), "B".into()],
        rows: vec![vec!["1".into(), "2".into()]],
    })]);
    let addr = Address::Root(0);
    assert!(table_insert_row(&mut d, addr, 1));
    assert!(table_insert_col(&mut d, addr, 2));
    match &d.blocks[0].kind {
        BlockKind::Table { header, rows } => {
            assert_eq!(header.len(), 3);
            assert_eq!(rows.len(), 2);
            assert!(rows.iter().all(|r| r.len() == 3));
        }
        _ => unreachable!(),
    }
    assert!(table_remove_col(&mut d, addr, 2));
    assert!(table_remove_row(&mut d, addr, 1));
    // Last row / last column never removed.
    assert!(!table_remove_row(&mut d, addr, 0));
    assert!(table_remove_col(&mut d, addr, 0));
    assert!(!table_remove_col(&mut d, addr, 0));
}

#[test]
fn indent_clamps() {
    let mut d = doc(vec![Block::new(BlockKind::ListItem {
        style: ListStyle::Bullet,
        checked: None,
        indent: 0,
        md: "x".into(),
    })]);
    assert!(!indent_list(&mut d, Address::Root(0), -1));
    for _ in 0..10 {
        indent_list(&mut d, Address::Root(0), 1);
    }
    assert!(matches!(
        d.blocks[0].kind,
        BlockKind::ListItem { indent: 5, .. }
    ));
}

#[test]
fn shortcuts() {
    let hit = line_start_shortcut("# Title").unwrap();
    assert!(matches!(hit.kind, BlockKind::Heading { level: 1, ref md } if md == "Title"));
    assert_eq!(hit.prefix_len, 2);

    assert!(matches!(
        line_start_shortcut("- [x] done").unwrap().kind,
        BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(true),
            ..
        }
    ));
    assert!(matches!(
        line_start_shortcut("1. first").unwrap().kind,
        BlockKind::ListItem {
            style: ListStyle::Number,
            ..
        }
    ));
    assert!(matches!(
        line_start_shortcut("> quoted").unwrap().kind,
        BlockKind::Quote { .. }
    ));
    assert!(matches!(
        line_start_shortcut("```rust").unwrap().kind,
        BlockKind::Code { ref lang, .. } if lang == "rust"
    ));
    assert!(matches!(
        line_start_shortcut("---").unwrap().kind,
        BlockKind::Divider
    ));
    assert!(matches!(
        line_start_shortcut(":::warning").unwrap().kind,
        BlockKind::Admonition {
            tone: Tone::Warning,
            ..
        }
    ));

    // Non-shortcuts.
    assert!(line_start_shortcut("#x").is_none());
    assert!(line_start_shortcut("-x").is_none());
    assert!(line_start_shortcut("##### five").is_none());
    assert!(line_start_shortcut(":::nope").is_none());
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
