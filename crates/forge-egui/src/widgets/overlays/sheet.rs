//! Slide-in edge panel with a scrim — the egui sibling of `@forge/ui`'s
//! `Sheet`. Slides over `theme.motion.base`; Esc or a scrim click closes.

use crate::theme::{scrim, FontWeight, Theme};
use crate::widgets::primitives::{Glyph, IconButton};
use egui::{CornerRadius, Frame, InnerResponse, Margin, Rect, Sense, Stroke, Ui, UiBuilder, Vec2};

/// Which screen edge the sheet slides in from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    Left,
    #[default]
    Right,
}

pub struct Sheet<'a> {
    title: &'a str,
    side: Side,
    width: f32,
}

impl<'a> Sheet<'a> {
    pub fn new(title: &'a str) -> Sheet<'a> {
        Sheet {
            title,
            side: Side::Right,
            width: 380.0,
        }
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Render while `*open` (and while animating shut); flips it false on
    /// Esc, scrim click, or the × button.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        open: &mut bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let ctx = ui.ctx().clone();
        let t = Theme::of(&ctx);
        let id = egui::Id::new(("forge-sheet", self.title, self.side as u8));

        // 0..1 slide progress; keep painting while the close animation runs.
        let progress = ctx.animate_bool_with_time(id.with("anim"), *open, t.motion.base);
        if progress <= 0.0 {
            return None;
        }

        let screen = ctx.content_rect();
        let panel_rect = match self.side {
            Side::Right => Rect::from_min_size(
                egui::pos2(screen.max.x - self.width * progress, screen.min.y),
                Vec2::new(self.width, screen.height()),
            ),
            Side::Left => Rect::from_min_size(
                egui::pos2(screen.min.x - self.width * (1.0 - progress), screen.min.y),
                Vec2::new(self.width, screen.height()),
            ),
        };

        let mut close = false;
        let area = egui::Area::new(id)
            .kind(egui::UiKind::Modal)
            .sense(Sense::hover())
            .fixed_pos(screen.min)
            .order(egui::Order::Foreground)
            .interactable(true);
        let inner = area.show(&ctx, |ui| {
            // Scrim over everything below; a click on it closes the sheet.
            let mut backdrop = ui.new_child(
                UiBuilder::new()
                    .sense(Sense::CLICK | Sense::DRAG)
                    .max_rect(screen),
            );
            backdrop.set_min_size(screen.size());
            ui.painter()
                .rect_filled(screen, 0.0, scrim(&t).gamma_multiply(progress));
            if backdrop.response().clicked() {
                close = true;
            }

            // The panel itself; sensed so clicks don't fall through.
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(panel_rect)
                    .sense(Sense::CLICK | Sense::DRAG),
                |ui| {
                    Frame::new()
                        .fill(t.bg[4])
                        .stroke(Stroke::new(1.0, t.border.default))
                        .corner_radius(CornerRadius::ZERO)
                        .inner_margin(Margin::same(20))
                        .show(ui, |ui| {
                            ui.set_width(self.width - 40.0);
                            ui.set_min_height(screen.height() - 40.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(self.title)
                                        .font(t.font(
                                            ui.ctx(),
                                            FontWeight::SemiBold,
                                            t.type_scale.md,
                                        ))
                                        .color(t.fg[0]),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if IconButton::new(Glyph::Cross, "Close")
                                            .small(true)
                                            .show(ui)
                                            .clicked()
                                        {
                                            close = true;
                                        }
                                    },
                                );
                            });
                            ui.add_space(t.space.x(3.0));
                            body(ui)
                        })
                        .inner
                },
            )
            .inner
        });

        if *open && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            close = true;
        }
        if close {
            *open = false;
        }
        Some(InnerResponse::new(inner.inner, inner.response))
    }
}
