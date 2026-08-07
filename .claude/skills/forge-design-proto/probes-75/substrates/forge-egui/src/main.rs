mod forge;

use forge::Theme;

struct App {
    theme: Theme,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx);
        egui::TopBottomPanel::top("bar").show(ctx, |ui| {
            ui.label("opsview");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Services");
            ui.label("ingest — us-east-1 — ok");
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "opsview",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App { theme: Theme::dark() }))),
    )
}
