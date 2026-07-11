//! Forge-styled hover tooltip — a thin restyle of `on_hover_ui`. The outer
//! frame comes from the themed visuals (bg\[4\], default border, no shadow);
//! this adds the Forge text treatment and padding.

use crate::theme::{FontWeight, Theme};
use egui::{Margin, Response, Ui};

/// Attach a hover tooltip to any response: `tooltip(button.response, "…")`.
pub fn tooltip(response: Response, text: &str) -> Response {
    let text = text.to_owned();
    response.on_hover_ui(move |ui: &mut Ui| {
        let t = Theme::of(ui.ctx());
        // Visuals give the popup frame 4pt margins; pad up to 6×8 total.
        egui::Frame::new()
            .inner_margin(Margin::symmetric(4, 2))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(text)
                        .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                        .color(t.fg[1]),
                );
            });
    })
}
