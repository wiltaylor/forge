//! Link preview card: title + optional description + url; clicking opens the
//! link in the system browser (scheme-sanitized like markdown links).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::specialty::markdown::safe_url;
use egui::{CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui};

pub struct LinkCard<'a> {
    title: &'a str,
    url: &'a str,
    description: Option<&'a str>,
}

impl<'a> LinkCard<'a> {
    pub fn new(title: &'a str, url: &'a str) -> LinkCard<'a> {
        LinkCard {
            title,
            url,
            description: None,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let frame = Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.default))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new(self.title)
                        .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                        .color(t.accent.fg),
                );
                if let Some(description) = self.description {
                    ui.label(
                        RichText::new(description)
                            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                            .color(t.fg[2]),
                    );
                }
                ui.label(
                    RichText::new(self.url)
                        .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs))
                        .color(t.fg[3]),
                );
            });

        let response = ui.interact(
            frame.response.rect,
            frame.response.id.with("forge-link-card"),
            Sense::click(),
        );
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let mut outcome = Outcome::Ignored;
        if response.clicked() {
            if let Some(url) = safe_url(self.url) {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                outcome = Outcome::Submitted;
            }
        }
        ForgeResponse::new(response, outcome)
    }
}
