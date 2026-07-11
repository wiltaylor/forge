//! Bordered surface container with an optional title row.

use crate::theme::{FontWeight, Theme};
use egui::{CornerRadius, Frame, InnerResponse, Margin, Stroke, Ui};

pub struct Card<'a> {
    title: Option<&'a str>,
    padded: bool,
}

impl<'a> Card<'a> {
    pub fn new() -> Card<'a> {
        Card {
            title: None,
            padded: true,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn padded(mut self, padded: bool) -> Self {
        self.padded = padded;
        self
    }

    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::of(ui.ctx());
        let margin = if self.padded {
            Margin::same(t.space.x(4.0) as i8)
        } else {
            Margin::ZERO
        };
        Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.lg as u8))
            .inner_margin(margin)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if let Some(title) = self.title {
                    ui.label(
                        egui::RichText::new(title)
                            .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                            .color(t.fg[0]),
                    );
                    ui.add_space(t.space.x(2.0));
                }
                body(ui)
            })
    }
}

impl Default for Card<'_> {
    fn default() -> Self {
        Card::new()
    }
}
