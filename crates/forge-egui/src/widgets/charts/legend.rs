//! Swatch + label legend row in palette order — sits beside whichever chart
//! shares the series ordering.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{series_color, FontWeight, Theme};
use egui::{Sense, Ui, Vec2};

pub struct Legend<'a> {
    labels: &'a [&'a str],
    wrap: bool,
}

impl<'a> Legend<'a> {
    pub fn new(labels: &'a [&'a str]) -> Legend<'a> {
        Legend {
            labels,
            wrap: false,
        }
    }

    /// Wrap onto extra rows instead of overflowing.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let items = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing = Vec2::new(16.0, 6.0);
            for (i, label) in self.labels.iter().enumerate() {
                let color = series_color(&t, i);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                    ui.label(
                        egui::RichText::new(*label)
                            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                            .color(t.fg[1]),
                    );
                });
            }
        };
        let response = if self.wrap {
            ui.horizontal_wrapped(items).response
        } else {
            ui.horizontal(items).response
        };
        ForgeResponse::new(response, Outcome::Ignored)
    }
}
