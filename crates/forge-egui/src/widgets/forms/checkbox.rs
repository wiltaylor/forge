//! Checkbox with an indeterminate visual, custom-painted for token-exact
//! states.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType};

const BOX_SIDE: f32 = 16.0;
const GAP: f32 = 8.0;

pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: &'a str,
    indeterminate: bool,
    disabled: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: &'a str) -> Checkbox<'a> {
        Checkbox {
            checked,
            label,
            indeterminate: false,
            disabled: false,
        }
    }

    /// Mixed state ("some of the children checked") — overrides the checked
    /// visual; clicking still toggles the bound bool.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base);
        let text_color = if self.disabled { t.fg[3] } else { t.fg[0] };
        let galley = util::galley(ui, self.label, font, text_color);

        let size = Vec2::new(
            BOX_SIDE + GAP + galley.size().x,
            galley.size().y.max(BOX_SIDE) + 4.0,
        );
        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);

        let mut outcome = Outcome::Ignored;
        if response.clicked() && !self.disabled {
            *self.checked = !*self.checked;
            outcome = Outcome::Changed;
        }
        let checked = *self.checked;
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Checkbox, !self.disabled, checked, self.label)
        });

        if ui.is_rect_visible(rect) {
            let box_rect = Rect::from_center_size(
                egui::pos2(rect.min.x + BOX_SIDE / 2.0, rect.center().y),
                Vec2::splat(BOX_SIDE),
            );
            let radius = CornerRadius::same(t.radius.sm as u8);
            let filled = checked || self.indeterminate;
            let (fill, border) = if self.disabled {
                (t.bg[2], Some(t.border.subtle))
            } else if filled {
                (t.accent.base, None)
            } else {
                let b = if response.hovered() {
                    t.accent.base
                } else {
                    t.border.strong
                };
                (t.bg[2], Some(b))
            };
            ui.painter().rect_filled(box_rect, radius, fill);
            if let Some(border) = border {
                ui.painter().rect_stroke(
                    box_rect,
                    radius,
                    Stroke::new(1.0, border),
                    StrokeKind::Inside,
                );
            }
            util::focus_ring(ui, &response, box_rect, t.radius.sm, &t);
            if filled {
                let glyph = if self.indeterminate {
                    Glyph::Minus
                } else {
                    Glyph::Check
                };
                let mark = if self.disabled {
                    t.fg[3]
                } else {
                    t.accent.contrast
                };
                let g = util::galley(
                    ui,
                    glyph.as_str(),
                    t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.sm),
                    mark,
                );
                ui.painter()
                    .galley(box_rect.center() - g.size() / 2.0, g, mark);
            }
            ui.painter().galley(
                egui::pos2(
                    rect.min.x + BOX_SIDE + GAP,
                    rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                text_color,
            );
        }

        ForgeResponse::new(response, outcome)
    }
}
