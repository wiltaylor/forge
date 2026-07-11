//! Message composer: a growing multiline draft where Enter sends and
//! Shift+Enter inserts a newline, plus a send button. `Submitted` carries
//! commit semantics — the caller reads and clears the bound `String`.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::{Glyph, IconButton, Variant};
use egui::{CornerRadius, Key, KeyboardShortcut, Margin, Modifiers, RichText, Stroke, Ui};

const MAX_ROWS: usize = 5;

/// `Composer::new(&mut draft).show(ui)`; on `.submitted()` take the draft.
pub struct Composer<'a> {
    text: &'a mut String,
    placeholder: &'a str,
}

impl<'a> Composer<'a> {
    pub fn new(text: &'a mut String) -> Composer<'a> {
        Composer {
            text,
            placeholder: "Message… (Enter send · Shift+Enter newline)",
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        // Grow 1 → MAX_ROWS with the draft.
        let rows = self.text.split('\n').count().clamp(1, MAX_ROWS);
        let mut send_clicked = false;

        let inner = ui.horizontal(|ui| {
            let send_w = t.control.md + ui.spacing().item_spacing.x;
            let well_w = (ui.available_width() - send_w).max(80.0);

            let mut prepared = egui::Frame::new()
                .fill(t.bg[1])
                .corner_radius(CornerRadius::same(t.radius.md as u8))
                .inner_margin(Margin::symmetric(10, 6))
                .begin(ui);
            let response = {
                let ui = &mut prepared.content_ui;
                ui.set_width(well_w - 20.0);
                let te = egui::TextEdit::multiline(self.text)
                    .frame(egui::Frame::NONE)
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY)
                    .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                    .text_color(t.fg[0])
                    .hint_text(RichText::new(self.placeholder).color(t.fg[3]))
                    // Shift+Enter inserts the newline; plain Enter is ours.
                    .return_key(Some(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter)));
                ui.add(te)
            };
            prepared.frame.stroke = Stroke::new(
                1.0,
                if response.has_focus() {
                    t.accent.base
                } else {
                    t.border.default
                },
            );
            prepared.end(ui);

            if IconButton::new(Glyph::ChevronRight, "Send")
                .variant(Variant::Primary)
                .show(ui)
                .submitted()
            {
                send_clicked = true;
            }
            response
        });
        let response = inner.inner;

        let mut outcome = Outcome::Ignored;
        if response.gained_focus() {
            outcome = Outcome::Consumed;
        }
        if response.changed() {
            outcome = outcome.merge(Outcome::Changed);
        }
        let enter_send = response.has_focus()
            && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift && !i.modifiers.ctrl);
        if (enter_send || send_clicked) && !self.text.trim().is_empty() {
            outcome = outcome.merge(Outcome::Submitted);
        }
        ForgeResponse::new(response, outcome)
    }
}
