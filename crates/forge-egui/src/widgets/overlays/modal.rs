//! Centered modal dialog — wraps `egui::Modal` (backdrop + Esc + input
//! blocking) with Forge chrome: theme scrim, bg\[4\] panel, title row with a
//! close button, and an optional footer slot.

use crate::theme::{scrim, FontWeight, Theme};
use crate::widgets::primitives::{Glyph, IconButton};
use egui::{CornerRadius, Frame, InnerResponse, Margin, Stroke, Ui};

/// Modal panel widths, mirroring `@forge/ui`'s `size` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModalWidth {
    #[default]
    Md,
    Lg,
    Xl,
}

impl ModalWidth {
    pub fn points(self) -> f32 {
        match self {
            ModalWidth::Md => 480.0,
            ModalWidth::Lg => 720.0,
            ModalWidth::Xl => 960.0,
        }
    }
}

pub struct Modal<'a> {
    title: &'a str,
    width: ModalWidth,
    #[allow(clippy::type_complexity)]
    footer: Option<Box<dyn FnOnce(&mut Ui) + 'a>>,
}

impl<'a> Modal<'a> {
    pub fn new(title: &'a str) -> Modal<'a> {
        Modal {
            title,
            width: ModalWidth::Md,
            footer: None,
        }
    }

    pub fn width(mut self, width: ModalWidth) -> Self {
        self.width = width;
        self
    }

    /// Actions row painted after a subtle separator below the body.
    pub fn footer(mut self, footer: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.footer = Some(Box::new(footer));
        self
    }

    /// Render while `*open`; flips it false on Esc, scrim click, or the ×
    /// button. Returns the body's result while shown.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        if !*open {
            return None;
        }
        let t = Theme::of(ctx);
        let width = self.width.points();
        let mut close_clicked = false;

        let modal = egui::Modal::new(egui::Id::new(("forge-modal", self.title)))
            .backdrop_color(scrim(&t))
            .frame(
                Frame::new()
                    .fill(t.bg[4])
                    .stroke(Stroke::new(1.0, t.border.default))
                    .corner_radius(CornerRadius::same(t.radius.lg as u8))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_width(width - 40.0); // inner width, padding excluded
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(self.title)
                            .font(t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.md))
                            .color(t.fg[0]),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if IconButton::new(Glyph::Cross, "Close")
                            .small(true)
                            .show(ui)
                            .clicked()
                        {
                            close_clicked = true;
                        }
                    });
                });
                ui.add_space(t.space.x(3.0));
                let inner = body(ui);
                if let Some(footer) = self.footer {
                    ui.add_space(t.space.x(4.0));
                    let y = ui.cursor().min.y;
                    ui.painter().line_segment(
                        [
                            egui::pos2(ui.max_rect().min.x, y),
                            egui::pos2(ui.max_rect().max.x, y),
                        ],
                        Stroke::new(1.0, t.border.subtle),
                    );
                    ui.add_space(t.space.x(4.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), footer);
                }
                inner
            });

        if close_clicked || modal.should_close() {
            *open = false;
        }
        Some(InnerResponse::new(modal.inner, modal.response))
    }
}
