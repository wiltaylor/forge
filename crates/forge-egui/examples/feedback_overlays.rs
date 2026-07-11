//! Visual tour of the M4 feedback + overlay widgets: alerts, progress,
//! spinners, toasts, modal, sheet, popover, tooltip, and menus.

use forge_egui::prelude::*;

struct Demo {
    progress: f32,
    modal_open: bool,
    modal_xl_open: bool,
    sheet_open: bool,
    name: String,
}

impl Default for Demo {
    fn default() -> Demo {
        Demo {
            progress: 0.65,
            modal_open: false,
            modal_xl_open: false,
            sheet_open: false,
            name: String::new(),
        }
    }
}

impl App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    self.body(ui, ctx);
                });
        });
    }
}

impl Demo {
    fn body(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx) {
        Card::new().title("Alerts").show(ui, |ui| {
            let _ = Alert::new(Tone::Info, "Heads up")
                .message("A new agent version is available.")
                .show(ui);
            ui.add_space(8.0);
            let _ = Alert::new(Tone::Success, "Deployed")
                .message("Build 421 rolled out to all regions.")
                .show(ui);
            ui.add_space(8.0);
            let _ = Alert::new(Tone::Warning, "Certificate expiring")
                .message("The TLS certificate for edge-2 expires in 6 days.")
                .show(ui);
            ui.add_space(8.0);
            let _ = Alert::new(Tone::Danger, "Sync failed")
                .message("Replica lag exceeded the failover threshold.")
                .show(ui);
        });
        ui.add_space(12.0);

        Card::new().title("Progress & spinners").show(ui, |ui| {
            let mut value = f64::from(self.progress);
            let _ = Slider::new(&mut value, 0.0..=1.0)
                .label("Drive it")
                .show(ui);
            self.progress = value as f32;
            ui.add_space(10.0);
            let _ = Progress::new(self.progress)
                .label("Upload")
                .show_value(true)
                .show(ui);
            ui.add_space(10.0);
            let _ = Progress::new(self.progress)
                .tone(Tone::Success)
                .label("Success tone")
                .show_value(true)
                .show(ui);
            ui.add_space(10.0);
            let _ = Progress::new(0.0)
                .indeterminate(true)
                .label("Indexing…")
                .show(ui);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 24.0;
                let _ = Spinner::new().size(12.0).show(ui);
                let _ = Spinner::new().show(ui);
                let _ = Spinner::new().size(24.0).show(ui);
                let _ = Spinner::new().label("Connecting…").show(ui);
            });
        });
        ui.add_space(12.0);

        Card::new().title("Toasts").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let _ = Toast::new(Severity::Info, "Snapshot scheduled").show(ui);
                let _ = Toast::new(Severity::Success, "Saved").show(ui);
                let _ = Toast::new(Severity::Warning, "Disk 85% full").show(ui);
                let _ = Toast::new(Severity::Danger, "Connection lost").show(ui);
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if Button::new("Push info toast").show(ui).clicked() {
                    ctx.toast().info("For your information");
                }
                if Button::new("Push error toast")
                    .variant(Variant::Danger)
                    .show(ui)
                    .clicked()
                {
                    ctx.toast().error("That broke something");
                }
            });
        });
        ui.add_space(12.0);

        Card::new().title("Overlays").show(ui, |ui| {
            ui.horizontal(|ui| {
                if Button::new("Modal (md)").show(ui).clicked() {
                    self.modal_open = true;
                }
                if Button::new("Modal (xl, footer)").show(ui).clicked() {
                    self.modal_xl_open = true;
                }
                if Button::new("Sheet").show(ui).clicked() {
                    self.sheet_open = true;
                }
                let _ = Popover::new("demo-popover").width(260.0).show(
                    ui,
                    |ui| Button::new("Popover").show(ui),
                    |ui| {
                        ui.label("Anchored floating panel.");
                        ui.add_space(6.0);
                        let _ = Progress::new(0.4).label("Inline widget").show(ui);
                    },
                );
                let r = Button::new("Hover me").variant(Variant::Ghost).show(ui);
                let _ = tooltip(r.response, "Forge-styled tooltip");
            });
            ui.add_space(8.0);

            let items = [
                MenuItem::new("Copy").icon(Glyph::File),
                MenuItem::new("Duplicate").icon(Glyph::Plus),
                MenuItem::new("Locked").disabled(true),
                MenuItem::new("Delete")
                    .icon(Glyph::Cross)
                    .danger(true)
                    .separator_before(true),
            ];
            ui.horizontal(|ui| {
                if let Some(index) =
                    DropdownMenu::new(&items).show(ui, |ui| Button::new("Dropdown ▾").show(ui))
                {
                    ctx.toast().info(format!("Menu: {}", items[index].label));
                }
                let t = Theme::of(ui.ctx());
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(220.0, 32.0), egui::Sense::click());
                ui.painter().rect_filled(rect, t.radius.md, t.bg[2]);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Right-click zone",
                    t.mono(t.type_scale.sm),
                    t.fg[2],
                );
                if let Some(index) = context_menu(&response, &items) {
                    ctx.toast().info(format!("Context: {}", items[index].label));
                }
            });
        });

        let _ = Modal::new("Modal title").show(ui.ctx(), &mut self.modal_open, |ui| {
            ui.label("Body content — Esc, scrim, or × closes.");
            ui.add_space(8.0);
            let _ = Input::new(&mut self.name)
                .label("Name")
                .placeholder("Focus works inside")
                .show(ui);
        });

        let mut xl_close = false;
        let _ = Modal::new("Wide modal")
            .width(ModalWidth::Xl)
            .footer(|ui| {
                if Button::new("Save")
                    .variant(Variant::Primary)
                    .show(ui)
                    .clicked()
                {
                    xl_close = true;
                }
                let _ = Button::new("Cancel").variant(Variant::Ghost).show(ui);
            })
            .show(ui.ctx(), &mut self.modal_xl_open, |ui| {
                let _ = Alert::new(Tone::Info, "960pt wide")
                    .message("The footer sits below a subtle separator.")
                    .show(ui);
            });
        if xl_close {
            self.modal_xl_open = false;
        }

        let _ = Sheet::new("Detail sheet").show(ui, &mut self.sheet_open, |ui| {
            ui.label("Slides in over the content.");
            ui.add_space(8.0);
            let _ = Progress::new(0.8)
                .label("Detail view")
                .show_value(true)
                .show(ui);
        });
    }
}

fn main() -> forge_egui::Result<()> {
    forge_egui::run(
        Demo::default(),
        Theme::dark(),
        RunOptions {
            window_title: "Forge — feedback & overlays".to_owned(),
            ..Default::default()
        },
    )
}
