//! Standalone status strip for apps not using the runtime Shell (which has
//! its own).

use crate::theme::Theme;
use egui::Ui;

pub struct StatusBar<'a> {
    left: &'a str,
    right: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    pub fn new(left: &'a str) -> StatusBar<'a> {
        StatusBar { left, right: None }
    }

    pub fn right(mut self, right: &'a str) -> Self {
        self.right = Some(right);
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        egui::Frame::new()
            .fill(t.bg[1])
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let rect = ui.max_rect().expand2(egui::vec2(12.0, 6.0));
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    egui::Stroke::new(1.0, t.border.subtle),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(self.left)
                            .size(t.type_scale.sm)
                            .color(t.fg[2]),
                    );
                    if let Some(right) = self.right {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(right)
                                    .size(t.type_scale.sm)
                                    .color(t.fg[2]),
                            );
                        });
                    }
                });
            })
            .response
    }
}
