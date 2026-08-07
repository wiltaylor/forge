//! A deploy screen, built with the Forge egui kit in `src/forge`.
//!
//! Keys: `F` toggles the in-flight state, `T` switches between the dark and the light
//! theme. Both are here so the screen can be checked in each state without a backend.

mod forge;

use egui::{Align, Layout, RichText, Stroke};

use forge::theme::{FontWeight, ThemeMode};
use forge::{Button, Theme, Variant};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Deploy",
        options,
        Box::new(|cc| {
            let app = DeployApp::default();
            app.theme.apply(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

struct DeployApp {
    theme: Theme,
    /// True while a deploy is in flight.
    in_flight: bool,
    /// The last action taken, shown under the row.
    status: String,
}

impl Default for DeployApp {
    fn default() -> Self {
        Self {
            theme: Theme::dark(),
            in_flight: false,
            status: "Idle. Nothing has been deployed in this session.".to_owned(),
        }
    }
}

impl eframe::App for DeployApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.read_keys(ui.ctx());
        let theme = self.theme;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.bg[0])
                    .inner_margin(egui::Margin::same(theme.space.x(6) as i8)),
            )
            .show(ui, |ui| {
                page_head(ui, &theme, "Deploy", "orca-api, production");
                ui.add_space(theme.space.x(5));
                self.action_row(ui, &theme);
                ui.add_space(theme.space.x(4));
                ui.label(
                    RichText::new(&self.status)
                        .font(theme.font(ui.ctx(), FontWeight::Regular, theme.type_scale.base))
                        .color(theme.fg[2]),
                );
                ui.add_space(theme.space.x(6));
                self.hints(ui, &theme);
            });
    }
}

impl DeployApp {
    /// The three actions of the screen, in one row.
    fn action_row(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.space.x(2);

            if Button::new("Deploy")
                .variant(Variant::Primary)
                .loading(self.in_flight)
                .show(ui)
                .submitted()
            {
                self.in_flight = true;
                self.status = "Deploy started. The release is in flight.".to_owned();
            }

            if Button::new("Dry run")
                .disabled(self.in_flight)
                .show(ui)
                .submitted()
            {
                self.status = "Dry run complete. 4 changes, 0 destructive.".to_owned();
            }

            if Button::new("Cancel deployment")
                .variant(Variant::Danger)
                .show(ui)
                .submitted()
            {
                self.in_flight = false;
                self.status = "Deploy cancelled. The release was rolled back.".to_owned();
            }
        });
    }

    fn hints(&self, ui: &mut egui::Ui, theme: &Theme) {
        let small = theme.font(ui.ctx(), FontWeight::Regular, theme.type_scale.sm);
        let state = if self.in_flight { "in flight" } else { "idle" };
        ui.label(
            RichText::new(format!("F — toggle the deploy state. Now: {state}."))
                .font(small.clone())
                .color(theme.fg[2]),
        );
        let mode = match theme.mode {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        };
        ui.label(
            RichText::new(format!("T — switch the theme. Now: {mode}."))
                .font(small)
                .color(theme.fg[2]),
        );
    }

    /// `F` and `T`. Enter and Space belong to the focused button, so neither is used here.
    fn read_keys(&mut self, ctx: &egui::Context) {
        let (flight, theme) =
            ctx.input(|i| (i.key_pressed(egui::Key::F), i.key_pressed(egui::Key::T)));
        if flight {
            self.in_flight = !self.in_flight;
            self.status = if self.in_flight {
                "Deploy started. The release is in flight.".to_owned()
            } else {
                "Idle. The deploy finished.".to_owned()
            };
        }
        if theme {
            self.theme = match self.theme.mode {
                ThemeMode::Dark => Theme::light(),
                ThemeMode::Light => Theme::dark(),
            };
            self.theme.apply(ctx);
        }
    }
}

/// The title band of the screen.
///
/// `reference/impl/egui/page-head.md` is not written, so this is the minimum
/// `reference/laws.md` asks for: an eyebrow, the title, and a 1px rule under it. It is a
/// local helper, not the Forge `page-head` control.
fn page_head(ui: &mut egui::Ui, theme: &Theme, title: &str, eyebrow: &str) {
    ui.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.label(
            RichText::new(eyebrow.to_uppercase())
                .font(theme.font(ui.ctx(), FontWeight::Medium, theme.type_scale.xs))
                .color(theme.fg[2]),
        );
        ui.add_space(theme.space.x(1));
        ui.label(
            RichText::new(title)
                .font(theme.font(ui.ctx(), FontWeight::SemiBold, theme.type_scale.h3))
                .color(theme.fg[0]),
        );
        ui.add_space(theme.space.x(3));
        let rule = ui.available_rect_before_wrap();
        ui.painter().hline(
            rule.left()..=rule.right(),
            rule.top(),
            Stroke::new(1.0, theme.border.default),
        );
    });
}
