//! Interactive question prompts — the chat kit's `ChatPrompt`: an AI asks,
//! the user answers through buttons / radio / checkboxes / a select. Buttons
//! submit immediately; the other controls pair with a Submit button.

use crate::theme::{FontWeight, Theme};
use crate::widgets::forms::{Checkbox, RadioGroup, Select, SelectState};
use crate::widgets::primitives::{Button, Variant};
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};

/// Which control the prompt renders.
#[derive(Clone, Debug, PartialEq)]
pub enum PromptControl {
    Buttons(Vec<String>),
    Radio(Vec<String>),
    Checkbox(Vec<String>),
    Select(Vec<String>),
}

/// The question plus its answer control.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatPromptData {
    pub question: String,
    pub control: PromptControl,
}

impl ChatPromptData {
    pub fn new(question: impl Into<String>, control: PromptControl) -> ChatPromptData {
        ChatPromptData {
            question: question.into(),
            control,
        }
    }
}

/// App-owned control state (plain data, headless-testable).
#[derive(Clone, Debug, Default)]
pub struct ChatPromptState {
    /// Radio selection.
    pub choice: Option<usize>,
    /// Checkbox states (resized to the option count on show).
    pub checks: Vec<bool>,
    /// Select dropdown state.
    pub select: SelectState,
}

/// The committed answer, returned once from [`ChatPrompt::show`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptAnswer {
    Button(usize),
    Radio(usize),
    Checks(Vec<usize>),
    Select(usize),
}

/// `ChatPrompt::new(&data).show(ui, &mut state)` → `Some(answer)` on submit.
#[derive(Clone, Debug)]
pub struct ChatPrompt<'a> {
    data: &'a ChatPromptData,
}

impl<'a> ChatPrompt<'a> {
    pub fn new(data: &'a ChatPromptData) -> ChatPrompt<'a> {
        ChatPrompt { data }
    }

    pub fn show(self, ui: &mut Ui, state: &mut ChatPromptState) -> Option<PromptAnswer> {
        let t = Theme::of(ui.ctx());
        let mut answer = None;
        Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.default))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 24.0);
                ui.label(
                    RichText::new(&self.data.question)
                        .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                        .color(t.fg[0]),
                );
                ui.add_space(t.space.x(2.0));
                match &self.data.control {
                    PromptControl::Buttons(options) => {
                        ui.horizontal_wrapped(|ui| {
                            for (i, option) in options.iter().enumerate() {
                                if Button::new(option).small(true).show(ui).submitted() {
                                    answer = Some(PromptAnswer::Button(i));
                                }
                            }
                        });
                    }
                    PromptControl::Radio(options) => {
                        let refs: Vec<&str> = options.iter().map(String::as_str).collect();
                        // `usize::MAX` = nothing selected yet.
                        let mut value = state.choice.unwrap_or(usize::MAX);
                        if RadioGroup::new(&mut value, &refs).show(ui).changed() {
                            state.choice = Some(value);
                        }
                        if submit_row(ui, state.choice.is_none()) {
                            answer = state.choice.map(PromptAnswer::Radio);
                        }
                    }
                    PromptControl::Checkbox(options) => {
                        state.checks.resize(options.len(), false);
                        for (i, option) in options.iter().enumerate() {
                            let _ = Checkbox::new(&mut state.checks[i], option).show(ui);
                        }
                        if submit_row(ui, !state.checks.iter().any(|c| *c)) {
                            answer = Some(PromptAnswer::Checks(
                                state
                                    .checks
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, c)| c.then_some(i))
                                    .collect(),
                            ));
                        }
                    }
                    PromptControl::Select(options) => {
                        let refs: Vec<&str> = options.iter().map(String::as_str).collect();
                        let _ = Select::new(&mut state.select, &refs)
                            .placeholder("Choose…")
                            .width(220.0)
                            .show(ui);
                        if submit_row(ui, state.select.value.is_none()) {
                            answer = state.select.value.map(PromptAnswer::Select);
                        }
                    }
                }
            });
        answer
    }
}

fn submit_row(ui: &mut Ui, disabled: bool) -> bool {
    let t = Theme::of(ui.ctx());
    ui.add_space(t.space.x(2.0));
    Button::new("Submit")
        .variant(Variant::Primary)
        .small(true)
        .disabled(disabled)
        .show(ui)
        .submitted()
}
