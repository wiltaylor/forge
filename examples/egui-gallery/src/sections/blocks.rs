//! Blocks: the Notion-style block page editor over the shared forge-blocks
//! document model (feature `blocks`), including a registered custom block.

use forge_egui::forge_blocks::sample::sample_document;
use forge_egui::prelude::*;
use forge_egui::widgets::{BlockEditor, BlockEditorState, CustomBlock};

/// A tiny consumer-defined block proving the [`CustomBlock`] trait: renders
/// its count with [-]/[+] buttons mutating `data["count"]`.
struct CounterBlock;

impl CustomBlock for CounterBlock {
    fn kind(&self) -> &'static str {
        "counter"
    }

    fn label(&self) -> &'static str {
        "Counter"
    }

    fn default_data(&self) -> serde_json::Value {
        serde_json::json!({ "count": 0 })
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut serde_json::Value,
        focused: bool,
        t: &forge_egui::theme::Theme,
    ) -> bool {
        let count = data["count"].as_i64().unwrap_or(0);
        let mut next = count;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Counter")
                    .font(t.mono(t.type_scale.sm))
                    .color(if focused { t.fg[0] } else { t.fg[2] }),
            );
            if Button::new("−").small(true).show(ui).clicked() {
                next -= 1;
            }
            ui.label(
                egui::RichText::new(count.to_string())
                    .font(t.mono(t.type_scale.md))
                    .color(t.accent.fg),
            );
            if Button::new("+").small(true).show(ui).clicked() {
                next += 1;
            }
        });
        if next != count {
            data["count"] = serde_json::json!(next);
            true
        } else {
            false
        }
    }
}

pub struct BlocksState {
    editor: BlockEditorState,
}

impl Default for BlocksState {
    fn default() -> BlocksState {
        let mut editor = BlockEditorState::new(sample_document());
        editor.register_custom(CounterBlock);
        BlocksState { editor }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut BlocksState) {
    let t = Theme::of(ui.ctx());
    PageHead::new("Blocks")
        .sub("Block page editor — click a block to edit its raw markdown source")
        .show(ui);
    let _ = BlockEditor::new(&mut state.editor).show(ui);
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new(
            "Enter split · Backspace-at-0 merge · Tab indent lists · Alt+↑/↓ move · \
             '/' on an empty block for the palette · ':ro' for emoji · Esc select",
        )
        .font(t.font(
            ui.ctx(),
            forge_egui::theme::FontWeight::Regular,
            t.type_scale.xs,
        ))
        .color(t.fg[2]),
    );
}
