//! Small shared paint helpers.

use crate::theme::Theme;
use egui::{CornerRadius, Rect, Response, Stroke, StrokeKind, Ui};

/// Paint the Forge focus ring around a focused widget: a 1.5pt accent
/// outline just outside the widget rect.
pub(crate) fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: f32, t: &Theme) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same((radius + 2.0) as u8),
            Stroke::new(1.5, t.accent.base),
            StrokeKind::Outside,
        );
    }
}

/// Layout a single-line galley in the given font/color.
pub(crate) fn galley(
    ui: &Ui,
    text: impl ToString,
    font: egui::FontId,
    color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout_no_wrap(text.to_string(), font, color)
}
