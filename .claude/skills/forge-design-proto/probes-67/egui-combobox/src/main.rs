//! A settings screen with a region picker, in the Forge style.

// Forge is a kit. The tokens and the parts this screen does not reach for are
// still part of the API, so the dead-code pass has nothing useful to say here.
#[allow(dead_code, unused_imports)]
mod forge;

use eframe::egui;

use forge::{
    shell::row_separator, AppShell, ComboBox, ComboBoxOption, ComboBoxState, Outcome, PageHead,
    SettingsLayout, SettingsRow, SettingsSection, Theme,
};

/// The regions this account can reach. The two marked `false` are not
/// available on this account, and they cannot be selected.
const REGIONS: [(&str, bool); 40] = [
    ("us-east-1", true),
    ("us-east-2", true),
    ("us-west-1", true),
    ("us-west-2", true),
    ("us-central-1", true),
    ("us-gov-east-1", true),
    ("us-gov-west-1", false),
    ("ca-central-1", true),
    ("ca-west-1", true),
    ("mx-central-1", true),
    ("sa-east-1", true),
    ("sa-south-1", true),
    ("eu-west-1", true),
    ("eu-west-2", true),
    ("eu-west-3", true),
    ("eu-central-1", true),
    ("eu-central-2", true),
    ("eu-north-1", true),
    ("eu-south-1", true),
    ("eu-south-2", true),
    ("uk-south-1", true),
    ("ch-north-1", true),
    ("af-south-1", true),
    ("af-north-1", true),
    ("me-central-1", true),
    ("me-south-1", true),
    ("il-central-1", true),
    ("ap-south-1", true),
    ("ap-south-2", true),
    ("ap-southeast-1", true),
    ("ap-southeast-2", true),
    ("ap-southeast-3", true),
    ("ap-southeast-4", true),
    ("ap-northeast-1", true),
    ("ap-northeast-2", true),
    ("ap-northeast-3", true),
    ("ap-east-1", true),
    ("cn-north-1", false),
    ("cn-northwest-1", true),
    ("nz-north-1", true),
];

struct SettingsApp {
    regions: Vec<ComboBoxOption<'static>>,
    region: ComboBoxState,
    /// The colour mode the theme was last installed for.
    installed: Option<egui::Theme>,
    last_outcome: Outcome,
}

impl SettingsApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            regions: REGIONS
                .iter()
                .map(|&(label, available)| ComboBoxOption::new(label).disabled(!available))
                .collect(),
            region: ComboBoxState::default(),
            installed: None,
            last_outcome: Outcome::Ignored,
        };
        app.sync_theme(&cc.egui_ctx);
        app
    }

    /// Forge is dark by default and defines a light theme. Follow the platform.
    fn sync_theme(&mut self, ctx: &egui::Context) {
        let mode = ctx.theme();
        if self.installed == Some(mode) {
            return;
        }
        match mode {
            egui::Theme::Dark => Theme::dark(),
            egui::Theme::Light => Theme::light(),
        }
        .apply(ctx);
        self.installed = Some(mode);
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.sync_theme(&ctx);
        let theme = Theme::get(&ctx);

        AppShell::new("Orca console").show(
            ui,
            |ui| {
                let font = theme.font(
                    ui.ctx(),
                    forge::FontWeight::Medium,
                    theme.type_scale.base,
                );
                ui.label(
                    egui::RichText::new("Settings")
                        .font(font)
                        .color(theme.accent.fg),
                );
            },
            |ui| {
                PageHead::new("Settings")
                    .eyebrow("Account")
                    .description("Where this account runs, and who it answers to.")
                    .show(ui);

                SettingsLayout::new().show(ui, |ui| {
                    SettingsSection::new("Placement")
                        .description("Two regions are not available on this account.")
                        .show(ui, |ui| {
                            SettingsRow::new("Region")
                                .help("Where new workloads start. Type to narrow the list.")
                                .show(ui, |ui| {
                                    let result = ComboBox::new("region-picker", &mut self.region)
                                        .options(&self.regions)
                                        .placeholder("Select a region")
                                        .empty_text("No region matches. Clear the text and try a shorter one.")
                                        .show(ui);
                                    if result.outcome != Outcome::Ignored {
                                        self.last_outcome = result.outcome;
                                    }
                                });

                            row_separator(ui);

                            SettingsRow::new("Selection")
                                .help("What the picker last reported.")
                                .show(ui, |ui| {
                                    let font = theme.mono(theme.type_scale.base);
                                    let selected = self
                                        .region
                                        .value
                                        .and_then(|value| self.regions.get(value))
                                        .map(|option| option.label)
                                        .unwrap_or("none");
                                    ui.label(
                                        egui::RichText::new(selected)
                                            .font(font)
                                            .color(theme.fg[1]),
                                    );
                                });
                        });
                });
            },
        );
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Forge settings",
        options,
        Box::new(|cc| Ok(Box::new(SettingsApp::new(cc)))),
    )
}
