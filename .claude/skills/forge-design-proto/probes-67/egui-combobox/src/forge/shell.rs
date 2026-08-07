//! app-shell, page-head, settings-layout, settings-section, settings-row — egui.
//!
//! The egui implementation pages for these five controls say "Not written", and
//! `reference/gaps.md` records no row for them. They are built here from
//! `reference/laws.md` alone:
//!
//! - every screen is `app-shell` > `page-head` > content;
//! - `page-head` carries the title and one primary action;
//! - a settings screen is `settings-layout` > `settings-section` >
//!   `settings-row`, one control per row, the label on the left, the control on
//!   the right, and the help text under the label;
//! - a form is one column at every width;
//! - related fields group under a heading, never inside a card.

use egui::{CentralPanel, Frame, Margin, Panel, RichText, Sense, Ui, Vec2};

use super::{
    response::{ForgeResponse, Outcome},
    theme::{FontWeight, Theme, SIDEBAR_WIDTH},
};

/// The shell every screen sits inside. Nothing renders outside it.
pub struct AppShell<'a> {
    product: &'a str,
}

impl<'a> AppShell<'a> {
    pub fn new(product: &'a str) -> Self {
        Self { product }
    }

    /// `sidebar` fills the nav rail; `content` fills the main region.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        sidebar: impl FnOnce(&mut Ui),
        content: impl FnOnce(&mut Ui) -> R,
    ) -> R {
        let ctx = ui.ctx().clone();
        let theme = Theme::get(&ctx);
        let product_font = theme.font(&ctx, FontWeight::SemiBold, theme.type_scale.md);

        Panel::left("app-shell-nav")
            .exact_size(SIDEBAR_WIDTH)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(theme.bg[1])
                    .inner_margin(Margin::same(theme.space.x(4.0) as i8)),
            )
            .show_separator_line(true)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(self.product)
                        .font(product_font)
                        .color(theme.fg[0]),
                );
                ui.add_space(theme.space.x(4.0));
                sidebar(ui);
            });

        CentralPanel::no_frame()
            .frame(
                Frame::new()
                    .fill(theme.bg[0])
                    .inner_margin(Margin::same(theme.space.x(6.0) as i8)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, content)
                    .inner
            })
            .inner
    }
}

/// The title of the screen, and at most one primary action.
pub struct PageHead<'a> {
    title: &'a str,
    eyebrow: Option<&'a str>,
    description: Option<&'a str>,
}

impl<'a> PageHead<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            eyebrow: None,
            description: None,
        }
    }

    /// The one all-caps string Forge allows.
    pub fn eyebrow(mut self, eyebrow: &'a str) -> Self {
        self.eyebrow = Some(eyebrow);
        self
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let theme = Theme::get(ui.ctx());
        let ctx = ui.ctx().clone();
        let title = theme.font(&ctx, FontWeight::SemiBold, theme.type_scale.h2);
        let eyebrow = theme.font(&ctx, FontWeight::Medium, theme.type_scale.xs);
        let body = theme.font(&ctx, FontWeight::Regular, theme.type_scale.base);

        let inner = ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = theme.space.x(1.0);
            if let Some(text) = self.eyebrow {
                ui.label(
                    RichText::new(text.to_uppercase())
                        .font(eyebrow)
                        .color(theme.fg[2]),
                );
            }
            ui.label(RichText::new(self.title).font(title).color(theme.fg[0]));
            if let Some(text) = self.description {
                ui.label(RichText::new(text).font(body).color(theme.fg[2]));
            }
        });

        ui.add_space(theme.space.x(6.0));
        ForgeResponse::new(inner.response, Outcome::Ignored)
    }
}

/// The body of a settings screen. One column, at every width.
pub struct SettingsLayout {
    max_width: f32,
}

impl Default for SettingsLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsLayout {
    pub fn new() -> Self {
        // A readable measure for one column, derived from the spacing step —
        // tokens.md has no width token for a form.
        Self { max_width: 180.0 }
    }

    /// The measure, in spacing steps.
    pub fn steps(mut self, steps: f32) -> Self {
        self.max_width = steps;
        self
    }

    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        let theme = Theme::get(ui.ctx());
        let width = theme.space.x(self.max_width).min(ui.available_width());
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = theme.space.x(6.0);
            content(ui)
        })
        .inner
    }
}

/// A group of related rows, under a heading. Never a card.
pub struct SettingsSection<'a> {
    heading: &'a str,
    description: Option<&'a str>,
}

impl<'a> SettingsSection<'a> {
    pub fn new(heading: &'a str) -> Self {
        Self {
            heading,
            description: None,
        }
    }

    pub fn description(mut self, description: &'a str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        let theme = Theme::get(ui.ctx());
        let ctx = ui.ctx().clone();
        let heading = theme.font(&ctx, FontWeight::SemiBold, theme.type_scale.lg);
        let body = theme.font(&ctx, FontWeight::Regular, theme.type_scale.sm);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = theme.space.x(2.0);
            ui.label(
                RichText::new(self.heading)
                    .font(heading)
                    .color(theme.fg[0]),
            );
            if let Some(text) = self.description {
                ui.label(RichText::new(text).font(body).color(theme.fg[2]));
            }
            ui.add_space(theme.space.x(1.0));
            let separator = ui.available_rect_before_wrap();
            ui.painter().hline(
                separator.x_range(),
                separator.top(),
                egui::Stroke::new(1.0, theme.border.subtle),
            );
            ui.add_space(theme.space.x(2.0));
            content(ui)
        })
        .inner
    }
}

/// One control. The label sits on the left, the control on the right, and the
/// help text under the label.
pub struct SettingsRow<'a> {
    label: &'a str,
    help: Option<&'a str>,
}

impl<'a> SettingsRow<'a> {
    pub fn new(label: &'a str) -> Self {
        Self { label, help: None }
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, control: impl FnOnce(&mut Ui) -> R) -> R {
        let theme = Theme::get(ui.ctx());
        let ctx = ui.ctx().clone();
        let label = theme.font(&ctx, FontWeight::Medium, theme.type_scale.base);
        let help = theme.font(&ctx, FontWeight::Regular, theme.type_scale.sm);

        // The label column, in spacing steps. tokens.md has no token for the
        // split of a settings row.
        let gap = theme.space.x(6.0);
        let total = ui.available_width();
        let label_w = theme.space.x(48.0).min((total - gap) / 2.0);

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;

            let inner = ui.allocate_ui_with_layout(
                Vec2::new(label_w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = theme.space.x(1.0);
                    ui.label(RichText::new(self.label).font(label).color(theme.fg[0]));
                    if let Some(text) = self.help {
                        ui.label(RichText::new(text).font(help).color(theme.fg[2]));
                    }
                },
            );
            let _ = inner;

            let control_w = (total - label_w - gap).max(0.0);
            ui.allocate_ui_with_layout(
                Vec2::new(control_w, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(control_w);
                    control(ui)
                },
            )
            .inner
        })
        .inner
    }
}

/// A hairline between rows.
pub fn row_separator(ui: &mut Ui) {
    let theme = Theme::get(ui.ctx());
    ui.add_space(theme.space.x(4.0));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0, theme.border.subtle),
    );
    ui.add_space(theme.space.x(4.0));
}
