//! Push buttons, custom-painted so hover/press colors come from Forge tokens
//! (egui's built-in `Button` derives its own tints, which drift off-palette).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{Color32, CornerRadius, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// Filled neutral chip — the default action look.
    #[default]
    Default,
    /// Solid accent fill; one per view.
    Primary,
    /// Borderless text button.
    Ghost,
    /// Solid danger fill for destructive actions.
    Danger,
}

pub struct Button<'a> {
    label: &'a str,
    variant: Variant,
    disabled: bool,
    small: bool,
    icon: Option<Glyph>,
    min_width: Option<f32>,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Button<'a> {
        Button {
            label,
            variant: Variant::Default,
            disabled: false,
            small: false,
            icon: None,
            min_width: None,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Compact height (`control.sm`).
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = Some(w);
        self
    }

    /// `(fill, text, border)` for the current interaction state.
    fn colors(
        &self,
        t: &Theme,
        hovered: bool,
        pressed: bool,
    ) -> (Color32, Color32, Option<Color32>) {
        if self.disabled {
            return (t.bg[2], t.fg[3], Some(t.border.subtle));
        }
        match self.variant {
            Variant::Primary => {
                let fill = if pressed {
                    t.accent.press
                } else if hovered {
                    t.accent.hover
                } else {
                    t.accent.base
                };
                (fill, t.accent.contrast, None)
            }
            Variant::Danger => {
                let fill = if pressed {
                    crate::theme::shift(t.danger.base, -0.12)
                } else if hovered {
                    crate::theme::shift(t.danger.base, 0.08)
                } else {
                    t.danger.base
                };
                (fill, t.accent.contrast, None)
            }
            Variant::Default => {
                let fill = if pressed {
                    t.bg[4]
                } else if hovered {
                    t.bg[3]
                } else {
                    t.bg[2]
                };
                (
                    fill,
                    t.fg[0],
                    Some(if hovered {
                        t.border.strong
                    } else {
                        t.border.default
                    }),
                )
            }
            Variant::Ghost => {
                let fill = if pressed {
                    t.bg[3]
                } else if hovered {
                    t.bg[2]
                } else {
                    Color32::TRANSPARENT
                };
                (fill, if hovered { t.fg[0] } else { t.fg[1] }, None)
            }
        }
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let height = if self.small {
            t.control.sm
        } else {
            t.control.md
        };
        let pad_x = if self.small { 10.0 } else { 12.0 };
        let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base);

        // Measure with a placeholder color; repaint with the state color below.
        let label_galley = util::galley(ui, self.label, font.clone(), Color32::WHITE);
        let icon_galley = self
            .icon
            .map(|g| util::galley(ui, g.as_str(), font.clone(), Color32::WHITE));
        let icon_w = icon_galley.as_ref().map_or(0.0, |g| g.size().x + 6.0);
        let mut width = label_galley.size().x + icon_w + pad_x * 2.0;
        if let Some(min) = self.min_width {
            width = width.max(min);
        }

        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click()
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Button, !self.disabled, self.label));

        if ui.is_rect_visible(rect) {
            let hovered = response.hovered();
            let pressed = response.is_pointer_button_down_on();
            let (fill, text, border) = self.colors(&t, hovered, pressed);
            let radius = CornerRadius::same(t.radius.md as u8);

            ui.painter().rect_filled(rect, radius, fill);
            if let Some(border) = border {
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    Stroke::new(1.0, border),
                    egui::StrokeKind::Inside,
                );
            }
            util::focus_ring(ui, &response, rect, t.radius.md, &t);

            let mut x = rect.min.x + pad_x;
            if let Some(icon) = self.icon {
                let g = util::galley(ui, icon.as_str(), font.clone(), text);
                let y = rect.center().y - g.size().y / 2.0;
                ui.painter().galley(egui::pos2(x, y), g, text);
                x += icon_w;
                let _ = icon;
            }
            let g = util::galley(ui, self.label, font, text);
            let y = rect.center().y - g.size().y / 2.0;
            ui.painter().galley(egui::pos2(x, y), g, text);
        }

        let outcome = if response.clicked() {
            Outcome::Submitted
        } else {
            Outcome::Ignored
        };
        ForgeResponse::new(response, outcome)
    }
}

/// A square icon-only button. `label` is required — it is the accessible
/// name (AccessKit) and the hover tooltip.
pub struct IconButton<'a> {
    glyph: Glyph,
    label: &'a str,
    variant: Variant,
    disabled: bool,
    small: bool,
}

impl<'a> IconButton<'a> {
    pub fn new(glyph: Glyph, label: &'a str) -> IconButton<'a> {
        IconButton {
            glyph,
            label,
            variant: Variant::Ghost,
            disabled: false,
            small: false,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let side = if self.small {
            t.control.sm
        } else {
            t.control.md
        };
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.md);

        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click()
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), sense);
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::Button, !self.disabled, self.label));

        if ui.is_rect_visible(rect) {
            let helper = Button {
                label: self.label,
                variant: self.variant,
                disabled: self.disabled,
                small: self.small,
                icon: None,
                min_width: None,
            };
            let (fill, text, border) =
                helper.colors(&t, response.hovered(), response.is_pointer_button_down_on());
            let radius = CornerRadius::same(t.radius.md as u8);
            ui.painter().rect_filled(rect, radius, fill);
            if let Some(border) = border {
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    Stroke::new(1.0, border),
                    egui::StrokeKind::Inside,
                );
            }
            util::focus_ring(ui, &response, rect, t.radius.md, &t);
            let g = util::galley(ui, self.glyph.as_str(), font, text);
            ui.painter().galley(rect.center() - g.size() / 2.0, g, text);
        }

        let response = response.on_hover_text(self.label);
        let outcome = if response.clicked() {
            Outcome::Submitted
        } else {
            Outcome::Ignored
        };
        ForgeResponse::new(response, outcome)
    }
}
