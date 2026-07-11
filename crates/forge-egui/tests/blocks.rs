#![cfg(feature = "blocks")]
//! Interaction-contract tests for the block page editor, driven headless
//! through egui_kittest: read-only rendering, click-to-edit raw source,
//! typed-draft commit, and the bundled emoji font.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use forge_egui::forge_blocks::{sample::sample_document, BlockKind, Document};
use forge_egui::prelude::*;
use forge_egui::widgets::{BlockEditor, BlockEditorState};
use std::cell::RefCell;

fn themed_harness<'a>(app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut harness = Harness::new_ui(app);
    Theme::dark().apply(&harness.ctx);
    harness.run();
    harness
}

#[test]
fn read_only_renders_sample_document() {
    let state = RefCell::new(BlockEditorState::new(sample_document()));
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = BlockEditor::new(&mut s).read_only(true).show(ui);
    });
    harness.run();
    // Inline markdown renders without the source markers, emoji resolved.
    let _ = harness.get_by_label_contains("Forge Blocks");
    let _ = harness.get_by_label("Blocks all the way down.");
    // Admonition, column cells, and custom placeholder show. (Table cells
    // are painted galleys — not part of the accessibility tree.)
    let _ = harness.get_by_label_contains("Careful");
    let _ = harness.get_by_label("Columns split content side by side.");
    let _ = harness.get_by_label("Custom block: counter");
    // No text inputs in read-only mode.
    assert!(harness
        .query_by_role(egui::accesskit::Role::MultilineTextInput)
        .is_none());
    drop(harness);
    // Read-only never mutates (ids are per-call, so compare the markdown
    // projection instead of block identity).
    assert_eq!(
        forge_egui::forge_blocks::to_markdown(&state.borrow().doc),
        forge_egui::forge_blocks::to_markdown(&sample_document())
    );
}

#[test]
fn click_focuses_paragraph_with_raw_markdown() {
    let doc = Document::from_blocks(vec![forge_egui::forge_blocks::Block::new(
        BlockKind::Paragraph {
            md: "plain **bold** tail".into(),
        },
    )]);
    let state = RefCell::new(BlockEditorState::new(doc));
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = BlockEditor::new(&mut s).show(ui);
    });
    // The unfocused label shows the styled text (no ** markers).
    harness.get_by_label("plain bold tail").click();
    harness.run();
    // Focused: a TextEdit carrying the raw markdown source appears.
    let node = harness.get_by_role(egui::accesskit::Role::MultilineTextInput);
    assert_eq!(node.value().as_deref(), Some("plain **bold** tail"));
}

#[test]
fn typing_into_focused_block_commits_to_document() {
    let doc = Document::from_blocks(vec![forge_egui::forge_blocks::Block::new(
        BlockKind::Paragraph { md: "hello".into() },
    )]);
    let state = RefCell::new(BlockEditorState::new(doc));
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = BlockEditor::new(&mut s).show(ui);
    });
    harness.get_by_label("hello").click();
    harness.run();
    let node = harness.get_by_role(egui::accesskit::Role::MultilineTextInput);
    node.type_text(" world");
    harness.run();
    drop(harness);
    let state = state.borrow();
    assert_eq!(
        state.doc.blocks[0].kind.md(),
        Some("hello world"),
        "typed text must commit into the document"
    );
}

#[test]
fn enter_splits_the_focused_block() {
    let doc = Document::from_blocks(vec![forge_egui::forge_blocks::Block::new(
        BlockKind::Paragraph { md: "ab".into() },
    )]);
    let state = RefCell::new(BlockEditorState::new(doc));
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = BlockEditor::new(&mut s).show(ui);
    });
    // Click focuses with the caret at the end; Enter splits there.
    harness.get_by_label("ab").click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    drop(harness);
    let state = state.borrow();
    assert_eq!(
        state.doc.blocks.len(),
        2,
        "Enter must split into two blocks"
    );
    assert_eq!(state.doc.blocks[0].kind.md(), Some("ab"));
    assert_eq!(
        state.doc.blocks[1].kind.md(),
        Some(""),
        "empty tail focused"
    );
}

#[test]
fn slash_palette_converts_an_empty_paragraph() {
    let state = RefCell::new(BlockEditorState::new(Document::new()));
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = BlockEditor::new(&mut s).show(ui);
    });
    // The empty paragraph renders as a one-space label; click to focus.
    harness.get_by_label(" ").click();
    harness.run();
    let node = harness.get_by_role(egui::accesskit::Role::MultilineTextInput);
    node.type_text("/head");
    harness.run();
    // Enter applies the highlighted palette row (first match: Heading 1).
    harness.key_press(egui::Key::Enter);
    harness.run();
    drop(harness);
    let state = state.borrow();
    assert!(
        matches!(
            state.doc.blocks[0].kind,
            BlockKind::Heading { level: 1, ref md } if md.is_empty()
        ),
        "expected Heading 1, got {:?}",
        state.doc.blocks[0].kind
    );
}

#[cfg(feature = "fonts")]
#[test]
fn emoji_font_is_installed_and_renders() {
    let mut harness = themed_harness(|ui| {
        ui.label("rocket \u{1F680} and sparkles \u{2728}");
        ui.monospace("mono \u{1F680}");
    });
    // A couple of frames so the queued font definitions land.
    harness.run();
    let (prop, mono) = harness.ctx.fonts(|f| {
        let d = f.definitions();
        (
            d.families
                .get(&egui::FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
            d.families
                .get(&egui::FontFamily::Monospace)
                .cloned()
                .unwrap_or_default(),
        )
    });
    assert!(
        prop.iter().any(|f| f == "noto-emoji"),
        "noto-emoji missing from the proportional chain: {prop:?}"
    );
    assert!(
        mono.iter().any(|f| f == "noto-emoji"),
        "noto-emoji missing from the monospace chain: {mono:?}"
    );
    // Real glyph coverage — would fail if the (variable) Noto Emoji TTF
    // didn't load. These chars exist ONLY in the full Noto Emoji, not in
    // egui's built-in emoji subset, so they must resolve from our font:
    // abacus, 1st-place medal, adult. (Common emoji like the rocket can't
    // be asserted through `has_glyph`: it reports a false negative whenever
    // a char resolves to the same face that owns the replacement glyph.)
    let ours_only = "\u{1F9EE}\u{1F947}\u{1F9D1}";
    harness.ctx.fonts_mut(|f| {
        assert!(
            f.has_glyphs(&egui::FontId::proportional(14.0), ours_only),
            "full-Noto-Emoji glyphs missing from the proportional chain"
        );
        assert!(
            f.has_glyphs(&egui::FontId::monospace(14.0), ours_only),
            "full-Noto-Emoji glyphs missing from the monospace chain"
        );
    });
}
