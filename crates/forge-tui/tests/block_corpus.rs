#![cfg(feature = "blocks")]
//! The block key corpus driven against the ratatui block editor.
//!
//! The corpus (`contract/blocks/corpus.json`) is authored data: a starting
//! document, an address, a key sequence, and the document that must result.
//! This driver is the adapter — it puts [`BlockEditorState`] at the address in
//! the case's mode, translates each `KeyboardEvent.code` into the crossterm
//! key the kit speaks, and hands the document back.

use forge_block_corpus::{Case, Key, Mode, RUST_TUI};
use forge_blocks::Document;
use forge_tui::widgets::BlockEditorState;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// `KeyboardEvent.code` (+ the produced character) → a crossterm key event.
/// Shift+Tab is `BackTab` on a terminal, which is why the code alone is not
/// enough to name the key.
fn key_event(key: &Key) -> KeyEvent {
    let mut mods = KeyModifiers::NONE;
    if key.shift {
        mods |= KeyModifiers::SHIFT;
    }
    if key.ctrl {
        mods |= KeyModifiers::CONTROL;
    }
    if key.alt {
        mods |= KeyModifiers::ALT;
    }
    let code = match key.code.as_str() {
        "Enter" | "NumpadEnter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Escape" => KeyCode::Esc,
        "Tab" if key.shift => KeyCode::BackTab,
        "Tab" => KeyCode::Tab,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        other => match key.char() {
            Some(c) => KeyCode::Char(c),
            None => panic!("no crossterm key for code {other:?}"),
        },
    };
    KeyEvent::new(code, mods)
}

/// Put the editor where the case says, press the case's keys, hand back the
/// document.
fn drive(case: &Case) -> Document {
    let mut state = BlockEditorState::new(case.document());
    let addr = case.at.address();
    match case.at.mode() {
        Mode::Select => state.select(addr),
        Mode::Text(caret) => assert!(
            state.edit(addr, caret),
            "{}: the block at {addr:?} takes no text caret",
            case.id
        ),
        Mode::Cell(row, col) => assert!(
            state.edit_cell(addr, row, col),
            "{}: the block at {addr:?} is not a table",
            case.id
        ),
    }
    for key in &case.keys {
        let _ = state.handle_key(key_event(key));
    }
    state.doc().clone()
}

#[test]
fn the_block_key_corpus_passes() {
    forge_block_corpus::run(RUST_TUI, drive);
}
