//! button — egui.
//!
//! Written from `reference/controls/button.md` and `reference/impl/egui/button.md`.
//!
//! `egui::Button` is not used: its padding, radius and hover model are not Forge's. The
//! rect is allocated at the control height, plus 12 horizontal padding and the galley
//! width, and painted in order — fill, 1px stroke, focus ring, content.

use std::f32::consts::TAU;

use egui::{
    epaint::StrokeKind, Color32, Painter, Pos2, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo,
    WidgetType,
};

use crate::forge::response::{ForgeResponse, Outcome};
use crate::forge::theme::{FontWeight, Theme};

/// Variant selects fill and stroke only. Geometry never changes with variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    /// One per screen. The accent solid.
    Primary,
    /// Everything else. The raised surface with a 1px border.
    #[default]
    Default,
    /// Toolbars and table rows. No fill until hover.
    Ghost,
    /// Destructive. The danger solid.
    Danger,
}

/// One action, labelled.
///
/// ```ignore
/// if Button::new("Deploy").variant(Variant::Primary).loading(busy).show(ui).submitted() {
///     // act
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Button {
    label: String,
    variant: Variant,
    disabled: bool,
    loading: bool,
    small: bool,
    full_width: bool,
}

impl Button {
    /// A button with a label. Every visible string is a parameter.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: Variant::Default,
            disabled: false,
            loading: false,
            small: false,
            full_width: false,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Activation while disabled is a no-op, and the widget still allocates its space.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Shows a spinner in the leading slot and takes the disabled path. The label stays
    /// mounted.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// The `sm` size. There is no large.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// Never full width unless the caller asks.
    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    /// Paint the button and report what happened.
    ///
    /// Returns `Outcome::Submitted` on activation, `Outcome::Ignored` otherwise.
    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let theme = Theme::from_ctx(ui.ctx());
        let enabled = !self.disabled && !self.loading;
        // `add_enabled_ui` keeps the space allocated; an early return would collapse the
        // layout around the button.
        ui.add_enabled_ui(enabled, |ui| self.paint(ui, &theme, enabled))
            .inner
    }

    fn paint(self, ui: &mut Ui, theme: &Theme, enabled: bool) -> ForgeResponse {
        let height = if self.small {
            theme.control.sm
        } else {
            theme.control.md
        };
        let pad_x = theme.space.x(3);
        let gap = theme.space.x(2);
        let font_size = if self.small {
            theme.type_scale.sm
        } else {
            theme.type_scale.base
        };
        let font = theme.font(ui.ctx(), FontWeight::Medium, font_size);

        // A painter of our own: the button paints its own disabled colours, so egui's
        // fade must not apply on top of them.
        let painter = Painter::new(ui.ctx().clone(), ui.layer_id(), ui.clip_rect());

        let text_color = self.text_color(theme, enabled, false, false);
        // Measure the galley regardless of state, so the width holds.
        let galley = painter.layout_no_wrap(self.label.clone(), font.clone(), text_color);

        let spinner_d = theme.type_scale.base;
        let leading = if self.loading { spinner_d + gap } else { 0.0 };
        let natural = pad_x * 2.0 + leading + galley.size().x;
        let width = if self.full_width {
            ui.available_width().max(natural)
        } else {
            natural
        };

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

        let hovered = response.hovered() && enabled;
        let pressed = response.is_pointer_button_down_on() && enabled;
        let focused = response.has_focus();

        // Enter and Space activate. egui routes both to the focused widget; the guard keeps
        // activation a no-op while disabled or loading.
        let key_activated = focused
            && enabled
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
        let activated = (response.clicked() && enabled) || key_activated;

        if ui.is_rect_visible(rect) {
            let corner = theme.corner(theme.radius.sm);

            // 1. fill and 2. the 1px stroke.
            painter.rect(
                rect,
                corner,
                self.fill(theme, enabled, hovered, pressed),
                self.stroke(theme, enabled),
                StrokeKind::Inside,
            );

            // 3. the focus ring — 2px accent, outside the rect, so content never shifts.
            if focused {
                painter.rect_stroke(
                    rect.expand(2.0),
                    theme.corner(theme.radius.sm + 2.0),
                    Stroke::new(2.0, theme.accent.base),
                    StrokeKind::Inside,
                );
            }

            // 4. the content.
            let content_color = self.text_color(theme, enabled, hovered, pressed);
            let mut x = rect.left() + pad_x;
            if self.loading {
                let centre = Pos2::new(x + spinner_d / 2.0, rect.center().y);
                spinner(
                    &painter,
                    centre,
                    spinner_d / 2.0,
                    theme.fg[2],
                    ui.input(|i| i.time),
                    theme.motion.slow,
                );
                ui.ctx().request_repaint();
                x += spinner_d + gap;
            }
            let galley = painter.layout_no_wrap(self.label.clone(), font, content_color);
            let text_pos = Pos2::new(x, rect.center().y - galley.size().y / 2.0);
            painter.galley(text_pos, galley, content_color);
        }

        // egui paints nothing into the accessibility tree for a hand-painted widget.
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &self.label));

        let outcome = if activated {
            Outcome::Submitted
        } else {
            Outcome::Ignored
        };
        ForgeResponse::new(response, outcome)
    }

    fn fill(&self, theme: &Theme, enabled: bool, hovered: bool, pressed: bool) -> Color32 {
        if !enabled {
            return match self.variant {
                Variant::Ghost => Color32::TRANSPARENT,
                _ => theme.bg[1],
            };
        }
        match self.variant {
            Variant::Primary => {
                if pressed {
                    theme.accent.press
                } else if hovered {
                    theme.accent.hover
                } else {
                    theme.accent.base
                }
            }
            Variant::Default => {
                if pressed {
                    theme.bg[3]
                } else if hovered {
                    theme.bg[2]
                } else {
                    theme.bg[1]
                }
            }
            Variant::Ghost => {
                if pressed {
                    theme.bg[2]
                } else if hovered {
                    theme.bg[1]
                } else {
                    Color32::TRANSPARENT
                }
            }
            Variant::Danger => {
                if pressed {
                    theme.danger.base.gamma_multiply(0.82)
                } else if hovered {
                    theme.danger.base.gamma_multiply(1.12)
                } else {
                    theme.danger.base
                }
            }
        }
    }

    fn stroke(&self, theme: &Theme, enabled: bool) -> Stroke {
        match (self.variant, enabled) {
            (Variant::Default, true) => Stroke::new(1.0, theme.border.default),
            (Variant::Default, false) => Stroke::new(1.0, theme.border.subtle),
            (_, true) => Stroke::NONE,
            (_, false) => Stroke::new(1.0, theme.border.subtle),
        }
    }

    fn text_color(&self, theme: &Theme, enabled: bool, _hovered: bool, _pressed: bool) -> Color32 {
        if !enabled {
            return theme.fg[3];
        }
        match self.variant {
            // `accent.contrast` is the "text on a solid" role; the token set has no
            // per-status contrast colour.
            Variant::Primary | Variant::Danger => theme.accent.contrast,
            Variant::Default | Variant::Ghost => theme.fg[0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use std::cell::Cell;

    /// A click activates, and the label reaches the accessibility tree.
    #[test]
    fn click_submits() {
        let hits = Cell::new(0);
        let mut harness = Harness::new_ui(|ui| {
            if Button::new("Deploy")
                .variant(Variant::Primary)
                .show(ui)
                .submitted()
            {
                hits.set(hits.get() + 1);
            }
        });
        harness.run();
        harness.get_by_label("Deploy").click();
        harness.run();
        assert_eq!(hits.get(), 1);
    }

    /// Activation while disabled is a no-op, and the space stays allocated.
    #[test]
    fn disabled_does_not_submit() {
        let hits = Cell::new(0);
        let mut harness = Harness::new_ui(|ui| {
            if Button::new("Dry run").disabled(true).show(ui).submitted() {
                hits.set(hits.get() + 1);
            }
        });
        harness.run();
        harness.get_by_label("Dry run").click();
        harness.run();
        assert_eq!(hits.get(), 0);
    }

    /// Loading takes the disabled path, and the label stays mounted.
    #[test]
    fn loading_does_not_submit() {
        let hits = Cell::new(0);
        let mut harness = Harness::new_ui(|ui| {
            if Button::new("Deploy")
                .variant(Variant::Primary)
                .loading(true)
                .show(ui)
                .submitted()
            {
                hits.set(hits.get() + 1);
            }
        });
        // The spinner repaints every frame, so the harness is stepped, not run to quiet.
        harness.run_steps(2);
        harness.get_by_label("Deploy").click();
        harness.run_steps(2);
        assert_eq!(hits.get(), 0);
    }

    /// The label is measured whatever the state, so the button keeps its label width.
    #[test]
    fn loading_keeps_the_label_width() {
        let plain = Cell::new(0.0_f32);
        let mut harness = Harness::new_ui(|ui| {
            plain.set(Button::new("Deploy").show(ui).response.rect.width());
        });
        harness.run();

        let busy = Cell::new(0.0_f32);
        let mut harness = Harness::new_ui(|ui| {
            busy.set(
                Button::new("Deploy")
                    .loading(true)
                    .show(ui)
                    .response
                    .rect
                    .width(),
            );
        });
        harness.run_steps(2);

        assert!(busy.get() > plain.get(), "the spinner takes a leading slot");
    }
}

/// The loading spinner — an arc at the icon stroke width, one turn per four slow steps.
fn spinner(painter: &Painter, centre: Pos2, radius: f32, colour: Color32, time: f64, slow: f32) {
    let period = (slow * 4.0) as f64;
    let start = ((time % period) / period) as f32 * TAU;
    let sweep = TAU * 0.72;
    let steps = 24;
    let points: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let angle = start + sweep * (i as f32 / steps as f32);
            centre + Vec2::new(angle.cos() * radius, angle.sin() * radius)
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(1.5, colour)));
}
