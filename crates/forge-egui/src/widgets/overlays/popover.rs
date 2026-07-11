//! Anchored floating panel below a trigger widget — richer than a tooltip,
//! lighter than a modal. Toggles on trigger click; click-outside/Esc dismiss.

use crate::response::ForgeResponse;
use crate::theme::Theme;
use egui::{CornerRadius, Frame, InnerResponse, Margin, Popup, PopupCloseBehavior, Stroke, Ui};

pub struct Popover {
    id_salt: egui::Id,
    width: f32,
}

impl Popover {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Popover {
        Popover {
            id_salt: egui::Id::new(id_salt),
            width: 280.0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Show the trigger; while toggled open, the panel floats below it.
    /// Returns the body's result while the popover is open.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        trigger: impl FnOnce(&mut Ui) -> ForgeResponse,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let t = Theme::of(ui.ctx());
        let response = trigger(ui);
        Popup::from_toggle_button_response(&response.response)
            .id(response.id.with(self.id_salt))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .gap(4.0)
            .width(self.width)
            .frame(
                Frame::new()
                    .fill(t.bg[4])
                    .stroke(Stroke::new(1.0, t.border.default))
                    .corner_radius(CornerRadius::same(t.radius.md as u8))
                    .inner_margin(Margin::same(12)),
            )
            .show(|ui| {
                ui.set_min_width(self.width - 26.0);
                body(ui)
            })
    }
}
