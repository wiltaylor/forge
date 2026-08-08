#![cfg(feature = "blocks")]
//! The block key corpus driven against the egui block editor.
//!
//! The corpus (`contract/blocks/corpus.json`) is authored data: a starting
//! document, an address, a key sequence, and the document that must result.
//! This driver is the adapter — it puts [`BlockEditorState`] at the address in
//! the case's mode, translates each `KeyboardEvent.code` into the egui key (or
//! text) the kit speaks, and hands the document back.
//!
//! egui decides a keypress over the *previous* frame's caret geometry, so each
//! key gets its own settle frames rather than one batch at the end.

use std::cell::RefCell;

use egui_kittest::Harness;
use forge_block_corpus::{Case, Key, Mode, RUST_EGUI};
use forge_egui::forge_blocks::Document;
use forge_egui::prelude::*;
use forge_egui::widgets::{BlockEditor, BlockEditorState};

/// Frames run after each key: one to apply the deferred actions, one for the
/// caret/focus they request to land before the next key arrives.
const SETTLE: usize = 2;

/// `KeyboardEvent.code` → the egui key, for the keys the editor reads. A
/// printable arrives as text instead (see [`press`]), so this only has to
/// cover the named ones.
fn egui_key(code: &str) -> Option<egui::Key> {
    Some(match code {
        "Enter" | "NumpadEnter" => egui::Key::Enter,
        "Backspace" => egui::Key::Backspace,
        "Delete" => egui::Key::Delete,
        "Escape" => egui::Key::Escape,
        "Tab" => egui::Key::Tab,
        "ArrowUp" => egui::Key::ArrowUp,
        "ArrowDown" => egui::Key::ArrowDown,
        "ArrowLeft" => egui::Key::ArrowLeft,
        "ArrowRight" => egui::Key::ArrowRight,
        "Home" => egui::Key::Home,
        "End" => egui::Key::End,
        _ => return None,
    })
}

/// Queue one corpus key. A key that produces a character is typed, because
/// that is what the focused `TextEdit` reads; everything else is a key event
/// with the case's modifiers.
fn press(harness: &Harness<'_>, key: &Key) {
    let modifiers = egui::Modifiers {
        alt: key.alt,
        ctrl: key.ctrl,
        shift: key.shift,
        mac_cmd: false,
        command: key.ctrl,
    };
    match egui_key(&key.code) {
        Some(k) => harness.key_press_modifiers(modifiers, k),
        None => {
            let text = key
                .key
                .clone()
                .unwrap_or_else(|| panic!("no egui key or text for code {:?}", key.code));
            harness.event(egui::Event::Text(text));
        }
    }
}

/// Put the editor where the case says, press the case's keys, hand back the
/// document.
fn drive(case: &Case) -> Document {
    let state = RefCell::new(BlockEditorState::new(case.document()));
    let addr = case.at.address();
    {
        let mut s = state.borrow_mut();
        match case.at.mode() {
            Mode::Select => s.select(addr),
            Mode::Text(caret) => assert!(
                s.edit(addr, caret),
                "{}: the block at {addr:?} takes no text caret",
                case.id
            ),
            Mode::Cell(row, col) => assert!(
                s.edit_cell(addr, row, col),
                "{}: the block at {addr:?} is not a table",
                case.id
            ),
        }
    }

    let mut harness = Harness::new_ui(|ui| {
        let mut s = state.borrow_mut();
        let _ = BlockEditor::new(&mut s).show(ui);
    });
    Theme::dark().apply(&harness.ctx);
    harness.run_steps(SETTLE);

    for key in &case.keys {
        press(&harness, key);
        harness.run_steps(SETTLE);
    }
    drop(harness);
    state.into_inner().doc
}

#[test]
fn the_block_key_corpus_passes() {
    forge_block_corpus::run(RUST_EGUI, drive);
}
