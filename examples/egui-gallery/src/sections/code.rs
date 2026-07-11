//! Code: syntax-highlighted viewer with annotations, and the line diff
//! (feature `code`).

use forge_egui::prelude::*;

const RUST_SAMPLE: &str = r#"use std::collections::HashMap;

/// Count word frequencies in a corpus.
fn word_counts(corpus: &str) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    let unused = 42;
    for word in corpus.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts.remove("");
    counts
}"#;

const DIFF_OLD: &str = r#"fn greet(name: &str) {
    println!("hello {name}");
    println!("goodbye");
}"#;

const DIFF_NEW: &str = r#"fn greet(name: &str) {
    println!("hello, {name}!");
    log::info!("greeted {name}");
    println!("goodbye");
}"#;

pub struct CodeSectionState {
    pub view: CodeViewState,
    annotations: Vec<CodeAnnotation>,
}

impl Default for CodeSectionState {
    fn default() -> CodeSectionState {
        CodeSectionState {
            view: CodeViewState::default(),
            annotations: vec![
                CodeAnnotation::new(6, Severity::Warning, "unused variable: `unused`"),
                CodeAnnotation::new(
                    10,
                    Severity::Danger,
                    "cannot remove from a borrowed map: `counts` is moved",
                ),
            ],
        }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut CodeSectionState) {
    Card::new().title("CodeView — annotations").show(ui, |ui| {
        let mut wrap = state.view.wrap;
        let _ = Checkbox::new(&mut wrap, "Soft-wrap long lines").show(ui);
        state.view.wrap = wrap;
        ui.add_space(6.0);
        let _ = CodeView::new(RUST_SAMPLE, "rs")
            .annotations(&state.annotations)
            .show_state(ui, &mut state.view);
        let t = Theme::of(ui.ctx());
        ui.label(
            egui::RichText::new("Hover an underlined line for the message · dots mark the gutter")
                .font(t.font(
                    ui.ctx(),
                    forge_egui::theme::FontWeight::Regular,
                    t.type_scale.xs,
                ))
                .color(t.fg[2]),
        );
    });
    ui.add_space(12.0);

    Card::new().title("DiffView").show(ui, |ui| {
        let _ = DiffView::new(DIFF_OLD, DIFF_NEW).show(ui);
    });
}
