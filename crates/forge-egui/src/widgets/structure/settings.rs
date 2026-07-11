//! Settings page scaffolding: a titled section of label/control rows.

use crate::theme::{FontWeight, Theme};
use egui::Ui;

pub struct SettingsSection<'a> {
    title: &'a str,
    sub: Option<&'a str>,
}

impl<'a> SettingsSection<'a> {
    pub fn new(title: &'a str) -> SettingsSection<'a> {
        SettingsSection { title, sub: None }
    }

    // Named for parity with the web `sub` prop.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(mut self, sub: &'a str) -> Self {
        self.sub = Some(sub);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
        let t = Theme::of(ui.ctx());
        ui.label(
            egui::RichText::new(self.title)
                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.md))
                .color(t.fg[0]),
        );
        if let Some(sub) = self.sub {
            ui.label(
                egui::RichText::new(sub)
                    .size(t.type_scale.sm)
                    .color(t.fg[2]),
            );
        }
        ui.add_space(t.space.x(2.0));
        let inner = egui::Frame::new()
            .fill(t.bg[1])
            .stroke(egui::Stroke::new(1.0, t.border.subtle))
            .corner_radius(egui::CornerRadius::same(t.radius.lg as u8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                body(ui)
            });
        ui.add_space(t.space.x(4.0));
        inner
    }
}

/// One settings row: label + help on the left, the control on the right.
pub struct SettingsRow<'a> {
    label: &'a str,
    help: Option<&'a str>,
}

impl<'a> SettingsRow<'a> {
    pub fn new(label: &'a str) -> SettingsRow<'a> {
        SettingsRow { label, help: None }
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn show(self, ui: &mut Ui, control: impl FnOnce(&mut Ui)) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let response = egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(self.label).color(t.fg[0]));
                        if let Some(help) = self.help {
                            ui.label(
                                egui::RichText::new(help)
                                    .size(t.type_scale.sm)
                                    .color(t.fg[2]),
                            );
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), control);
                });
            })
            .response;
        // Row divider (skipped naturally for the last row by the section frame).
        let rect = response.rect;
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, t.border.subtle),
        );
        response
    }
}
