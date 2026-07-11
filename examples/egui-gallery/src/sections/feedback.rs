//! Feedback: alerts in every tone, determinate + indeterminate progress,
//! spinners, standalone toast cards, and runtime toast triggers.

use forge_egui::prelude::*;

pub struct FeedbackState {
    progress: f32,
}

impl Default for FeedbackState {
    fn default() -> FeedbackState {
        FeedbackState { progress: 0.65 }
    }
}

pub fn draw(ui: &mut egui::Ui, ctx: &mut Ctx, state: &mut FeedbackState) {
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

    Card::new().title("Progress").show(ui, |ui| {
        let mut value = f64::from(state.progress);
        let _ = Slider::new(&mut value, 0.0..=1.0)
            .label("Drive it")
            .show(ui);
        state.progress = value as f32;
        ui.add_space(10.0);
        let _ = Progress::new(state.progress)
            .label("Upload")
            .show_value(true)
            .show(ui);
        ui.add_space(10.0);
        let _ = Progress::new(state.progress)
            .tone(Tone::Success)
            .label("Success tone")
            .show_value(true)
            .show(ui);
        ui.add_space(10.0);
        let _ = Progress::new(state.progress)
            .tone(Tone::Danger)
            .label("Danger tone")
            .show_value(true)
            .show(ui);
        ui.add_space(10.0);
        let _ = Progress::new(0.0)
            .indeterminate(true)
            .label("Indexing…")
            .show(ui);
    });
    ui.add_space(12.0);

    Card::new().title("Spinner").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 24.0;
            let _ = Spinner::new().size(12.0).show(ui);
            let _ = Spinner::new().show(ui);
            let _ = Spinner::new().size(24.0).show(ui);
            let _ = Spinner::new().size(16.0).label("Connecting…").show(ui);
        });
    });
    ui.add_space(12.0);

    Card::new()
        .title("Toast cards (standalone)")
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let _ = Toast::new(Severity::Info, "Snapshot scheduled").show(ui);
                let _ = Toast::new(Severity::Success, "Saved").show(ui);
                let _ = Toast::new(Severity::Warning, "Disk 85% full").show(ui);
                let _ = Toast::new(Severity::Danger, "Connection lost").show(ui);
            });
        });
    ui.add_space(12.0);

    Card::new().title("Runtime toaster").show(ui, |ui| {
        ui.horizontal(|ui| {
            if Button::new("Info").show(ui).clicked() {
                ctx.toast().info("For your information");
            }
            if Button::new("Success").show(ui).clicked() {
                ctx.toast().success("It worked");
            }
            if Button::new("Warning").show(ui).clicked() {
                ctx.toast().warning("Careful now");
            }
            if Button::new("Error")
                .variant(Variant::Danger)
                .show(ui)
                .clicked()
            {
                ctx.toast().error("That broke something");
            }
        });
    });
}
