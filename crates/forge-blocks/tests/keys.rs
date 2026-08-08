//! The shared editing policy: which operation a keypress means.
//!
//! The corpus (`contract/blocks/corpus.json`) proves the kits agree on the
//! resulting *documents*. These tests pin the decision itself — above all the
//! demote-before-merge rule, which used to be written once per kit.

use forge_blocks::*;

fn p(md: &str) -> Block {
    Block::new(BlockKind::Paragraph { md: md.into() })
}

fn doc(blocks: Vec<Block>) -> Document {
    Document::from_blocks(blocks)
}

fn heading(md: &str) -> Block {
    Block::new(BlockKind::Heading {
        level: 2,
        md: md.into(),
    })
}

fn item(md: &str, indent: u8) -> Block {
    Block::new(BlockKind::ListItem {
        style: ListStyle::Bullet,
        checked: None,
        indent,
        md: md.into(),
    })
}

fn table() -> Block {
    Block::new(BlockKind::Table {
        header: vec!["a".into(), "b".into()],
        rows: vec![vec!["1".into(), "2".into()]],
    })
}

/// Resolve `key` against block `i` with a text caret at `caret`.
fn at_text(d: &Document, i: usize, caret: usize, key: Key) -> Option<Op> {
    resolve_key(d, Address::Root(i), Focus::Text { caret }, &key)
}

fn at_cell(d: &Document, i: usize, row: usize, col: usize, key: Key) -> Option<Op> {
    resolve_key(d, Address::Root(i), Focus::Cell { row, col }, &key)
}

fn selected(d: &Document, i: usize, key: Key) -> Option<Op> {
    resolve_key(d, Address::Root(i), Focus::Select, &key)
}

/* ---------------- the key shape ----------------------------------------- */

#[test]
fn a_typed_character_carries_its_us_layout_code() {
    assert_eq!(Key::typed('a').code, "KeyA");
    assert_eq!(Key::typed('a').char(), Some('a'));
    assert!(!Key::typed('a').shift);
    assert_eq!(Key::typed('Q').code, "KeyQ");
    assert!(Key::typed('Q').shift);
    assert_eq!(Key::typed('#').code, "Digit3");
    assert!(Key::typed('#').shift);
    assert_eq!(Key::typed('/').code, "Slash");
    assert_eq!(Key::typed(' ').code, "Space");
    // A character the layout does not name still types itself.
    assert_eq!(Key::typed('é').code, "Unidentified");
    assert_eq!(Key::typed('é').char(), Some('é'));
}

#[test]
fn a_key_reads_back_in_a_report() {
    assert_eq!(Key::new("Tab").shift().label(), "Shift+Tab");
    assert_eq!(Key::typed('a').ctrl().label(), "Ctrl+KeyA \"a\"");
}

/* ---------------- demote before merge ----------------------------------- */

#[test]
fn backspace_at_zero_merges_a_paragraph() {
    let d = doc(vec![p("hello "), p("world")]);
    assert_eq!(
        at_text(&d, 1, 0, Key::new("Backspace")),
        Some(Op::Merge {
            addr: Address::Root(1)
        })
    );
}

#[test]
fn backspace_at_zero_demotes_every_other_text_kind_first() {
    for block in [
        heading("title"),
        item("item", 0),
        Block::new(BlockKind::Quote { md: "quote".into() }),
        Block::new(BlockKind::Admonition {
            tone: Tone::Info,
            title: String::new(),
            md: "body".into(),
        }),
    ] {
        let d = doc(vec![p("above"), block]);
        assert_eq!(
            at_text(&d, 1, 0, Key::new("Backspace")),
            Some(Op::Demote {
                addr: Address::Root(1)
            }),
            "{:?} demotes before it merges",
            d.blocks[1].kind
        );
    }
}

#[test]
fn backspace_inside_the_text_belongs_to_the_kit() {
    let d = doc(vec![p("above"), p("world")]);
    assert_eq!(at_text(&d, 1, 3, Key::new("Backspace")), None);
}

#[test]
fn delete_at_the_end_merges_the_block_below() {
    let d = doc(vec![p("hello"), p("world")]);
    assert_eq!(
        at_text(&d, 0, 5, Key::new("Delete")),
        Some(Op::Merge {
            addr: Address::Root(1)
        })
    );
    assert_eq!(at_text(&d, 0, 2, Key::new("Delete")), None);
    // Nothing below: bound, but with nothing to do.
    assert_eq!(at_text(&d, 1, 5, Key::new("Delete")), Some(Op::Nothing));
}

/* ---------------- text keys --------------------------------------------- */

#[test]
fn enter_splits_and_shift_enter_breaks_the_line() {
    let d = doc(vec![p("hello")]);
    assert_eq!(
        at_text(&d, 0, 2, Key::new("Enter")),
        Some(Op::Split { caret: 2 })
    );
    assert_eq!(
        at_text(&d, 0, 2, Key::new("Enter").shift()),
        Some(Op::Insert('\n'))
    );
    assert_eq!(
        at_text(&d, 0, 2, Key::new("Enter").alt()),
        Some(Op::Insert('\n'))
    );
}

#[test]
fn tab_indents_list_items_and_leaves_everything_else_alone() {
    let d = doc(vec![item("one", 0), p("two")]);
    assert_eq!(
        at_text(&d, 0, 0, Key::new("Tab")),
        Some(Op::Indent { delta: 1 })
    );
    assert_eq!(
        at_text(&d, 0, 0, Key::new("Tab").shift()),
        Some(Op::Indent { delta: -1 })
    );
    assert_eq!(at_text(&d, 1, 0, Key::new("Tab")), None);
}

#[test]
fn a_line_start_shortcut_converts_instead_of_typing() {
    let d = doc(vec![p("#")]);
    assert_eq!(
        at_text(&d, 0, 1, Key::typed(' ')),
        Some(Op::Convert {
            kind: BlockKind::Heading {
                level: 1,
                md: String::new()
            },
            caret: 0,
        })
    );
    // The text after the prefix rides along, and the caret keeps its place
    // in it.
    let d = doc(vec![p("#hi")]);
    assert_eq!(
        at_text(&d, 0, 1, Key::typed(' ')),
        Some(Op::Convert {
            kind: BlockKind::Heading {
                level: 1,
                md: "hi".into()
            },
            caret: 0,
        })
    );
}

#[test]
fn a_shortcut_prefix_is_just_text_away_from_the_start_or_off_a_paragraph() {
    let d = doc(vec![p("a#")]);
    assert_eq!(at_text(&d, 0, 2, Key::typed(' ')), Some(Op::Insert(' ')));
    let d = doc(vec![heading("-")]);
    assert_eq!(at_text(&d, 0, 1, Key::typed(' ')), Some(Op::Insert(' ')));
}

#[test]
fn slash_opens_the_palette_only_on_an_empty_block() {
    let d = doc(vec![p(""), p("text")]);
    assert_eq!(at_text(&d, 0, 0, Key::typed('/')), Some(Op::OpenPalette));
    assert_eq!(at_text(&d, 1, 4, Key::typed('/')), Some(Op::Insert('/')));
}

#[test]
fn alt_arrows_move_the_block_and_plain_arrows_do_not() {
    let d = doc(vec![p("one"), p("two")]);
    assert_eq!(
        at_text(&d, 0, 0, Key::new("ArrowDown").alt()),
        Some(Op::MoveBlock { dir: 1 })
    );
    assert_eq!(
        at_text(&d, 1, 0, Key::new("ArrowUp").alt()),
        Some(Op::MoveBlock { dir: -1 })
    );
    assert_eq!(at_text(&d, 0, 0, Key::new("ArrowDown")), None);
    assert_eq!(at_text(&d, 0, 0, Key::new("ArrowLeft")), None);
    assert_eq!(at_text(&d, 0, 0, Key::new("Home")), None);
}

#[test]
fn escape_drops_out_of_text_onto_the_block() {
    let d = doc(vec![p("one")]);
    assert_eq!(
        at_text(&d, 0, 1, Key::new("Escape")),
        Some(Op::Select {
            addr: Address::Root(0)
        })
    );
}

#[test]
fn ctrl_t_cycles_the_tone_and_other_chorded_keys_pass() {
    let d = doc(vec![p("one")]);
    assert_eq!(
        at_text(&d, 0, 0, Key::typed('t').ctrl()),
        Some(Op::CycleTone)
    );
    assert_eq!(at_text(&d, 0, 0, Key::typed('x').ctrl()), None);
    assert_eq!(at_text(&d, 0, 0, Key::typed('x').alt()), None);
}

/* ---------------- table cells ------------------------------------------- */

#[test]
fn tab_walks_the_cells_and_wraps() {
    let d = doc(vec![table()]);
    assert_eq!(
        at_cell(&d, 0, 0, 0, Key::new("Tab")),
        Some(Op::FocusCell { row: 0, col: 1 })
    );
    assert_eq!(
        at_cell(&d, 0, 0, 1, Key::new("Tab")),
        Some(Op::FocusCell { row: 1, col: 0 })
    );
    // Last cell of the last row wraps back to the header.
    assert_eq!(
        at_cell(&d, 0, 1, 1, Key::new("Tab")),
        Some(Op::FocusCell { row: 0, col: 0 })
    );
    assert_eq!(
        at_cell(&d, 0, 0, 1, Key::new("Tab").shift()),
        Some(Op::FocusCell { row: 0, col: 0 })
    );
    assert_eq!(
        at_cell(&d, 0, 0, 0, Key::new("Tab").shift()),
        Some(Op::FocusCell { row: 1, col: 1 })
    );
}

#[test]
fn enter_steps_down_a_table_and_grows_it_off_the_last_row() {
    let d = doc(vec![table()]);
    assert_eq!(
        at_cell(&d, 0, 0, 0, Key::new("Enter")),
        Some(Op::FocusCell { row: 1, col: 0 })
    );
    assert_eq!(
        at_cell(&d, 0, 1, 0, Key::new("Enter")),
        Some(Op::InsertRow {
            at: 1,
            focus: Some((2, 0))
        })
    );
    assert_eq!(
        at_cell(&d, 0, 1, 0, Key::new("Enter").ctrl()),
        Some(Op::InsertRow { at: 1, focus: None })
    );
}

#[test]
fn alt_plus_and_minus_add_and_drop_a_column() {
    let d = doc(vec![table()]);
    assert_eq!(
        at_cell(&d, 0, 0, 1, Key::typed('=').alt()),
        Some(Op::InsertCol { at: 2 })
    );
    assert_eq!(
        at_cell(&d, 0, 0, 1, Key::typed('-').alt()),
        Some(Op::RemoveCol { at: 1 })
    );
}

#[test]
fn a_cell_takes_typing_and_leaves_its_caret_to_the_kit() {
    let d = doc(vec![table()]);
    assert_eq!(at_cell(&d, 0, 0, 0, Key::typed('x')), Some(Op::Insert('x')));
    assert_eq!(at_cell(&d, 0, 0, 0, Key::new("Backspace")), None);
    assert_eq!(at_cell(&d, 0, 0, 0, Key::new("ArrowLeft")), None);
}

/* ---------------- a kit's own buffer ------------------------------------- */

#[test]
fn a_buffer_keeps_its_keys_but_not_the_block_ones() {
    let d = doc(vec![Block::new(BlockKind::Code {
        lang: "rust".into(),
        code: "fn main() {}".into(),
    })]);
    let buffer = |key: Key| resolve_key(&d, Address::Root(0), Focus::Buffer, &key);
    assert_eq!(
        buffer(Key::new("Escape")),
        Some(Op::Select {
            addr: Address::Root(0)
        })
    );
    assert_eq!(
        buffer(Key::new("ArrowUp").alt()),
        Some(Op::MoveBlock { dir: -1 })
    );
    assert_eq!(buffer(Key::new("ArrowUp")), None);
    assert_eq!(buffer(Key::new("Enter")), None);
    assert_eq!(buffer(Key::typed('x')), None);
}

/* ---------------- block selection --------------------------------------- */

#[test]
fn selection_steps_enters_and_removes() {
    let d = doc(vec![p("one"), p("two")]);
    assert_eq!(
        selected(&d, 0, Key::new("ArrowDown")),
        Some(Op::Select {
            addr: Address::Root(1)
        })
    );
    assert_eq!(selected(&d, 0, Key::new("ArrowUp")), Some(Op::Nothing));
    assert_eq!(selected(&d, 0, Key::new("Enter")), Some(Op::Enter));
    assert_eq!(selected(&d, 0, Key::new("Delete")), Some(Op::Remove));
    assert_eq!(selected(&d, 0, Key::new("Backspace")), Some(Op::Remove));
    assert_eq!(
        selected(&d, 0, Key::typed('c')),
        Some(Op::WrapColumns { n: 2 })
    );
    assert_eq!(selected(&d, 0, Key::typed('/')), Some(Op::OpenPalette));
    assert_eq!(selected(&d, 0, Key::typed('x')), None);
}

#[test]
fn escape_leaves_a_column_cell_before_it_leaves_the_editor() {
    let mut d = doc(vec![p("one")]);
    let cell = wrap_in_columns(&mut d, Address::Root(0), 2).unwrap();
    assert_eq!(
        resolve_key(&d, cell, Focus::Select, &Key::new("Escape")),
        Some(Op::Select {
            addr: Address::Root(0)
        })
    );
    assert_eq!(selected(&d, 0, Key::new("Escape")), Some(Op::Blur));
}

#[test]
fn a_selected_columns_container_steps_outside_itself() {
    let mut d = doc(vec![p("one"), p("two")]);
    wrap_in_columns(&mut d, Address::Root(0), 2).unwrap();
    // Root(0) is the container: it is not a navigation stop, so ↓ lands on
    // the block after it and ↑ has nowhere to go.
    assert_eq!(
        selected(&d, 0, Key::new("ArrowDown")),
        Some(Op::Select {
            addr: Address::Root(1)
        })
    );
    assert_eq!(selected(&d, 0, Key::new("ArrowUp")), Some(Op::Nothing));
}
