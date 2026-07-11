#![cfg(feature = "blocks")]
//! Block editor tests: glyph snapshots of the shared sample document plus
//! key-sequence checks of the editing model (split/merge/shortcuts/palette/
//! emoji/indent/move).

use forge_blocks::{sample::sample_document, Address, Block, BlockKind, Document, ListStyle};
use forge_tui::event::Outcome;
use forge_tui::theme::Theme;
use forge_tui::widgets::{BlockEditor, BlockEditorState};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

fn buffer_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in area.y..area.y + area.height {
        let mut line = String::new();
        for x in area.x..area.x + area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn render(state: &mut BlockEditorState, w: u16, h: u16, read_only: bool) -> Buffer {
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    BlockEditor::new()
        .theme(&t)
        .read_only(read_only)
        .focused(true)
        .render(Rect::new(0, 0, w, h), &mut buf, state);
    buf
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

fn type_str(state: &mut BlockEditorState, s: &str) {
    for c in s.chars() {
        let _ = state.handle_key(key(KeyCode::Char(c)));
    }
}

fn paragraph(md: &str) -> Block {
    Block::new(BlockKind::Paragraph { md: md.into() })
}

fn doc(blocks: Vec<Block>) -> Document {
    Document::from_blocks(blocks)
}

#[test]
fn sample_document_read_only_snapshot() {
    let mut state = BlockEditorState::new(sample_document());
    let buf = render(&mut state, 80, 30, true);
    insta::assert_snapshot!(buffer_text(&buf));
}

#[test]
fn focused_paragraph_shows_raw_source() {
    let mut state =
        BlockEditorState::new(doc(vec![paragraph("**bold** and *italic* with :rocket:")]));
    assert!(state.edit(Address::Root(0), 0));
    let buf = render(&mut state, 44, 3, false);
    insta::assert_snapshot!(buffer_text(&buf));
}

#[test]
fn enter_splits_the_block_at_the_caret() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("hello")]));
    assert!(state.edit(Address::Root(0), 2));
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    assert_eq!(state.doc().blocks.len(), 2);
    assert_eq!(state.doc().blocks[0].kind.md(), Some("he"));
    assert_eq!(state.doc().blocks[1].kind.md(), Some("llo"));
    assert_eq!(state.focus(), Some(Address::Root(1)));
    assert_eq!(state.caret(), Some(0));
}

#[test]
fn backspace_at_zero_merges_with_previous() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("hello "), paragraph("world")]));
    assert!(state.edit(Address::Root(1), 0));
    assert_eq!(state.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert_eq!(state.doc().blocks.len(), 1);
    assert_eq!(state.doc().blocks[0].kind.md(), Some("hello world"));
    assert_eq!(state.focus(), Some(Address::Root(0)));
    assert_eq!(state.caret(), Some(6));
}

#[test]
fn backspace_at_zero_demotes_heading_first() {
    let mut state = BlockEditorState::new(doc(vec![
        paragraph("a"),
        Block::new(BlockKind::Heading {
            level: 2,
            md: "title".into(),
        }),
    ]));
    assert!(state.edit(Address::Root(1), 0));
    assert_eq!(state.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert!(matches!(
        state.doc().blocks[1].kind,
        BlockKind::Paragraph { .. }
    ));
    assert_eq!(state.doc().blocks.len(), 2);
}

#[test]
fn heading_shortcut_converts_empty_paragraph() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("")]));
    assert!(state.edit(Address::Root(0), 0));
    type_str(&mut state, "# ");
    assert!(matches!(
        state.doc().blocks[0].kind,
        BlockKind::Heading { level: 1, .. }
    ));
    assert_eq!(state.caret(), Some(0));
    type_str(&mut state, "Title");
    assert_eq!(state.doc().blocks[0].kind.md(), Some("Title"));
}

#[test]
fn slash_palette_converts_the_block() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("")]));
    assert!(state.edit(Address::Root(0), 0));
    assert_eq!(state.handle_key(key(KeyCode::Char('/'))), Outcome::Consumed);
    assert!(state.popup_open());
    type_str(&mut state, "div");
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    assert!(!state.popup_open());
    assert!(matches!(state.doc().blocks[0].kind, BlockKind::Divider));
}

#[test]
fn alt_down_moves_the_block() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("first"), paragraph("second")]));
    state.select(Address::Root(0));
    assert_eq!(state.handle_key(alt(KeyCode::Down)), Outcome::Changed);
    assert_eq!(state.doc().blocks[0].kind.md(), Some("second"));
    assert_eq!(state.doc().blocks[1].kind.md(), Some("first"));
    assert_eq!(state.focus(), Some(Address::Root(1)));
}

#[test]
fn emoji_completion_inserts_the_shortcode_text() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("")]));
    assert!(state.edit(Address::Root(0), 0));
    type_str(&mut state, "go :ro");
    assert!(state.popup_open());
    type_str(&mut state, "cke");
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    assert!(!state.popup_open());
    // Storage keeps the shortcode; rendering resolves it to the glyph.
    assert_eq!(state.doc().blocks[0].kind.md(), Some("go :rocket:"));
}

#[test]
fn tab_indents_a_list_item() {
    let mut state = BlockEditorState::new(doc(vec![Block::new(BlockKind::ListItem {
        style: ListStyle::Bullet,
        checked: None,
        indent: 0,
        md: "item".into(),
    })]));
    assert!(state.edit(Address::Root(0), 0));
    assert_eq!(state.handle_key(key(KeyCode::Tab)), Outcome::Changed);
    assert!(matches!(
        state.doc().blocks[0].kind,
        BlockKind::ListItem { indent: 1, .. }
    ));
    let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(state.handle_key(backtab), Outcome::Changed);
    assert!(matches!(
        state.doc().blocks[0].kind,
        BlockKind::ListItem { indent: 0, .. }
    ));
}

#[test]
fn enter_on_a_table_edits_cell_zero_zero() {
    let mut state = BlockEditorState::new(doc(vec![Block::new(BlockKind::Table {
        header: vec!["a".into(), "b".into()],
        rows: vec![vec!["1".into(), "2".into()]],
    })]));
    state.select(Address::Root(0));
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Consumed);
    type_str(&mut state, "!");
    assert_eq!(state.handle_key(key(KeyCode::Tab)), Outcome::Consumed);
    match &state.doc().blocks[0].kind {
        BlockKind::Table { header, .. } => assert_eq!(header[0], "a!"),
        other => panic!("expected table, got {other:?}"),
    }
}

#[test]
fn esc_inside_a_column_selects_the_container() {
    let mut state = BlockEditorState::new(doc(vec![paragraph("solo")]));
    state.select(Address::Root(0));
    assert_eq!(state.handle_key(key(KeyCode::Char('c'))), Outcome::Changed);
    let cell = Address::Cell {
        root: 0,
        col: 0,
        idx: 0,
    };
    assert_eq!(state.focus(), Some(cell));
    assert_eq!(state.handle_key(key(KeyCode::Esc)), Outcome::Consumed);
    assert_eq!(state.focus(), Some(Address::Root(0)));
    assert!(matches!(
        state.doc().blocks[0].kind,
        BlockKind::Columns { .. }
    ));
}

#[test]
fn registered_custom_block_receives_keys() {
    struct Bump;
    impl forge_tui::widgets::CustomBlock for Bump {
        fn kind(&self) -> &'static str {
            "bump"
        }
        fn label(&self) -> &'static str {
            "Bump"
        }
        fn default_data(&self) -> serde_json::Value {
            serde_json::json!({ "n": 0 })
        }
        fn height(&self, _d: &serde_json::Value, _w: u16, _t: &Theme) -> u16 {
            1
        }
        fn render(
            &mut self,
            _d: &serde_json::Value,
            _area: Rect,
            _buf: &mut Buffer,
            _focused: bool,
            _t: &Theme,
        ) {
        }
        fn handle_key(&mut self, data: &mut serde_json::Value, key: KeyEvent) -> Outcome {
            if key.code == KeyCode::Char('+') {
                let n = data["n"].as_i64().unwrap_or(0);
                data["n"] = serde_json::json!(n + 1);
                return Outcome::Changed;
            }
            Outcome::Ignored
        }
    }
    let mut state = BlockEditorState::new(doc(vec![Block::new(BlockKind::Custom {
        kind: "bump".into(),
        data: serde_json::json!({ "n": 0 }),
    })]));
    state.register_custom(Box::new(Bump));
    state.select(Address::Root(0));
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Consumed);
    assert_eq!(state.handle_key(key(KeyCode::Char('+'))), Outcome::Changed);
    match &state.doc().blocks[0].kind {
        BlockKind::Custom { data, .. } => assert_eq!(data["n"], 1),
        other => panic!("expected custom, got {other:?}"),
    }
    // Esc leaves the custom block back to selection.
    assert_eq!(state.handle_key(key(KeyCode::Esc)), Outcome::Consumed);
    assert!(!state.is_editing());
}

#[test]
fn read_only_scrolls_but_never_edits() {
    let mut state = BlockEditorState::new(sample_document());
    let _ = render(&mut state, 80, 10, true);
    let before = state.doc().clone();
    assert_eq!(state.handle_key(key(KeyCode::Down)), Outcome::Consumed);
    assert_eq!(state.handle_key(key(KeyCode::Char('x'))), Outcome::Ignored);
    assert_eq!(*state.doc(), before);
}
