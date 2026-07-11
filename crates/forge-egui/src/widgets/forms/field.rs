//! Shared field chrome: the label row, help/error line, well border rules,
//! and the flyout frame + option row used by Select/Combobox.

use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{
    Color32, CornerRadius, Margin, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType,
};

pub(super) fn label_row(ui: &mut Ui, t: &Theme, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm))
            .color(t.fg[1]),
    );
    ui.add_space(t.space.x(1.0));
}

pub(super) fn sub_line(ui: &mut Ui, t: &Theme, text: &str, color: Color32) {
    ui.add_space(t.space.x(1.0));
    ui.label(
        egui::RichText::new(text)
            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
            .color(color),
    );
}

pub(super) fn well_border(t: &Theme, error: bool, focused: bool, disabled: bool) -> Color32 {
    if disabled {
        t.border.subtle
    } else if error {
        t.danger.base
    } else if focused {
        t.accent.base
    } else {
        t.border.default
    }
}

/// Popover surface for Select/Combobox flyouts: bg\[4\] over a 1pt border.
pub(super) fn flyout_frame(t: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(t.bg[4])
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(4))
}

/// One hoverable row inside a flyout. `highlighted` is the keyboard cursor.
pub(super) fn option_row(
    ui: &mut Ui,
    t: &Theme,
    text: &str,
    selected: bool,
    highlighted: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), t.control.sm),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, text));
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(t.radius.sm as u8);
        if response.hovered() || highlighted {
            ui.painter().rect_filled(rect, radius, t.bg[2]);
        }
        let color = if selected {
            t.accent.fg
        } else if response.hovered() {
            t.fg[0]
        } else {
            t.fg[1]
        };
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base);
        let g = util::galley(ui, text, font.clone(), color);
        ui.painter().galley(
            egui::pos2(rect.min.x + 8.0, rect.center().y - g.size().y / 2.0),
            g,
            color,
        );
        if selected {
            let g = util::galley(ui, Glyph::Check.as_str(), font, t.accent.fg);
            ui.painter().galley(
                egui::pos2(
                    rect.max.x - 8.0 - g.size().x,
                    rect.center().y - g.size().y / 2.0,
                ),
                g,
                t.accent.fg,
            );
        }
    }
    response
}

/// Keyboard highlight cursor for flyouts — lives in egui temp memory so the
/// public `FooState` structs stay plain data.
pub(super) fn highlight(ui: &Ui, id: egui::Id) -> usize {
    ui.ctx().data(|d| d.get_temp(id)).unwrap_or(0)
}

pub(super) fn set_highlight(ui: &Ui, id: egui::Id, value: usize) {
    ui.ctx().data_mut(|d| d.insert_temp(id, value));
}

/// ▾ chevron at the right edge of a picker field.
pub(super) fn chevron(ui: &Ui, t: &Theme, rect: Rect, color: Color32) {
    let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm);
    let g = util::galley(ui, Glyph::ChevronDown.as_str(), font, color);
    ui.painter().galley(
        egui::pos2(
            rect.max.x - 10.0 - g.size().x,
            rect.center().y - g.size().y / 2.0,
        ),
        g,
        color,
    );
}

/// Paint a picker field well (fill + state border).
pub(super) fn well(ui: &Ui, t: &Theme, rect: Rect, border: Color32) {
    let radius = CornerRadius::same(t.radius.md as u8);
    ui.painter().rect_filled(rect, radius, t.bg[1]);
    ui.painter()
        .rect_stroke(rect, radius, Stroke::new(1.0, border), StrokeKind::Inside);
}
