//! Single-line text field: a Forge-painted well wrapping `egui::TextEdit`
//! (egui keeps cursor/selection/IME state; the chrome and Outcome are ours).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::forms::field;
use crate::widgets::primitives::{Glyph, Icon};
use egui::{CornerRadius, Key, Margin, Stroke, Ui, Vec2};

pub struct Input<'a> {
    text: &'a mut String,
    label: Option<&'a str>,
    help: Option<&'a str>,
    error: Option<&'a str>,
    placeholder: Option<&'a str>,
    masked: bool,
    icon: Option<Glyph>,
    desired_width: Option<f32>,
    disabled: bool,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Input<'a> {
        Input {
            text,
            label: None,
            help: None,
            error: None,
            placeholder: None,
            masked: false,
            icon: None,
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

    /// Password-style masking.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
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
                    .inner_margin(Margin::symmetric(10, 0))
                    .begin(ui);
                let response = {
                    let ui = &mut prepared.content_ui;
                    ui.set_width(width - 20.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if let Some(glyph) = self.icon {
                            let _ = Icon::new(glyph).color(t.fg[2]).show(ui);
                        }
                        let mut te = egui::TextEdit::singleline(self.text)
                            .frame(egui::Frame::NONE)
                            .password(self.masked)
                            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                            .text_color(if self.disabled { t.fg[3] } else { t.fg[0] })
                            .vertical_align(egui::Align::Center)
                            .min_size(Vec2::new(0.0, t.control.md))
                            .desired_width(ui.available_width())
                            .interactive(!self.disabled);
                        if let Some(p) = self.placeholder {
                            te = te.hint_text(egui::RichText::new(p).color(t.fg[3]));
                        }
                        ui.add(te)
                    })
                    .inner
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
        if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            outcome = outcome.merge(Outcome::Submitted);
        }
        ForgeResponse::new(response, outcome)
    }
}
