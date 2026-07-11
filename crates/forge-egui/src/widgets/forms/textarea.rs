//! Multi-line text field — same chrome as [`Input`](super::Input), wrapping
//! `egui::TextEdit::multiline`. Ctrl+Enter submits (plain Enter is a newline).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::forms::field;
use egui::{CornerRadius, Key, Margin, Stroke, Ui};

pub struct Textarea<'a> {
    text: &'a mut String,
    label: Option<&'a str>,
    help: Option<&'a str>,
    error: Option<&'a str>,
    placeholder: Option<&'a str>,
    rows: usize,
    desired_width: Option<f32>,
    disabled: bool,
}

impl<'a> Textarea<'a> {
    pub fn new(text: &'a mut String) -> Textarea<'a> {
        Textarea {
            text,
            label: None,
            help: None,
            error: None,
            placeholder: None,
            rows: 4,
            desired_width: None,
            disabled: false,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    /// Validation message; overrides `help` and turns the border danger.
    pub fn error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Visible rows (default 4).
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let width = self.desired_width.unwrap_or(240.0);

        let response = ui
            .vertical(|ui| {
                ui.set_max_width(width);
                if let Some(label) = self.label {
                    field::label_row(ui, &t, label);
                }

                let mut prepared = egui::Frame::new()
                    .fill(t.bg[1])
                    .corner_radius(CornerRadius::same(t.radius.md as u8))
                    .inner_margin(Margin::symmetric(10, 8))
                    .begin(ui);
                let response = {
                    let ui = &mut prepared.content_ui;
                    ui.set_width(width - 20.0);
                    let mut te = egui::TextEdit::multiline(self.text)
                        .frame(egui::Frame::NONE)
                        .desired_rows(self.rows)
                        .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                        .text_color(if self.disabled { t.fg[3] } else { t.fg[0] })
                        .desired_width(ui.available_width())
                        .interactive(!self.disabled);
                    if let Some(p) = self.placeholder {
                        te = te.hint_text(egui::RichText::new(p).color(t.fg[3]));
                    }
                    ui.add(te)
                };
                prepared.frame.stroke = Stroke::new(
                    1.0,
                    field::well_border(
                        &t,
                        self.error.is_some(),
                        response.has_focus(),
                        self.disabled,
                    ),
                );
                prepared.end(ui);

                if let Some(error) = self.error {
                    field::sub_line(ui, &t, error, t.danger.base);
                } else if let Some(help) = self.help {
                    field::sub_line(ui, &t, help, t.fg[2]);
                }
                response
            })
            .inner;

        let mut outcome = Outcome::Ignored;
        if response.gained_focus() {
            outcome = Outcome::Consumed;
        }
        if response.changed() {
            outcome = outcome.merge(Outcome::Changed);
        }
        if response.has_focus() && ui.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Enter)) {
            outcome = outcome.merge(Outcome::Submitted);
        }
        ForgeResponse::new(response, outcome)
    }
}
