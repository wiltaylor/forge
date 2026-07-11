//! Centered empty state: icon, title, hint, optional action row.

use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use egui::Ui;

pub struct Empty<'a> {
    title: &'a str,
    message: Option<&'a str>,
    icon: Glyph,
}

impl<'a> Empty<'a> {
    pub fn new(title: &'a str) -> Empty<'a> {
        Empty {
            title,
            message: None,
            icon: Glyph::Circle,
        }
    }

    pub fn message(mut self, message: &'a str) -> Self {
        self.message = Some(message);
        self
    }

    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = icon;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        self.show_with(ui, |_| {})
    }

    /// With an action row (typically a [`Button`](crate::widgets::Button))
    /// under the hint.
    pub fn show_with(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> egui::Response {
        let t = Theme::of(ui.ctx());
        ui.vertical_centered(|ui| {
            ui.add_space(t.space.x(8.0));
            ui.label(
                egui::RichText::new(self.icon.as_str())
                    .size(t.type_scale.h2)
                    .color(t.fg[3]),
            );
            ui.add_space(t.space.x(2.0));
            ui.label(
                egui::RichText::new(self.title)
                    .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.md))
                    .color(t.fg[1]),
            );
            if let Some(message) = self.message {
                ui.add_space(t.space.x(1.0));
                ui.label(
                    egui::RichText::new(message)
                        .size(t.type_scale.sm)
                        .color(t.fg[2]),
                );
            }
            ui.add_space(t.space.x(3.0));
            actions(ui);
            ui.add_space(t.space.x(8.0));
        })
        .response
    }
}
