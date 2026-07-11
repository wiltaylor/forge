//! Forms: text inputs, checks, toggles, radios, sliders, and segments — all
//! bound to live state with mono readouts proving the binding.

use forge_egui::prelude::*;

pub struct FormsState {
    name: String,
    email: String,
    password: String,
    search: String,
    notes: String,
    agree: bool,
    mixed: bool,
    notifications: bool,
    airplane: bool,
    radio: usize,
    segment: usize,
    volume: f64,
    threshold: f64,
}

impl Default for FormsState {
    fn default() -> FormsState {
        FormsState {
            name: String::new(),
            email: "not-an-email".into(),
            password: String::new(),
            search: String::new(),
            notes: String::new(),
            agree: true,
            mixed: false,
            notifications: true,
            airplane: false,
            radio: 0,
            segment: 1,
            volume: 40.0,
            threshold: 0.75,
        }
    }
}

pub fn draw(ui: &mut egui::Ui, s: &mut FormsState) {
    let t = Theme::of(ui.ctx());

    Card::new().title("Text inputs").show(ui, |ui| {
        ui.columns(3, |cols| {
            let _ = Input::new(&mut s.name)
                .label("Name")
                .placeholder("Jane Doe")
                .help("Shown on your profile")
                .show(&mut cols[0]);
            let _ = Input::new(&mut s.email)
                .label("Email")
                .placeholder("you@example.com")
                .error("Enter a valid email address")
                .show(&mut cols[1]);
            let _ = Input::new(&mut s.password)
                .label("Password")
                .placeholder("••••••••")
                .masked(true)
                .show(&mut cols[2]);
        });
        ui.add_space(12.0);
        ui.columns(3, |cols| {
            let _ = Input::new(&mut s.search)
                .label("Search")
                .placeholder("Search hosts…")
                .icon(Glyph::Search)
                .show(&mut cols[0]);
            let _ = Textarea::new(&mut s.notes)
                .label("Notes")
                .placeholder("Ctrl+Enter submits")
                .rows(4)
                .show(&mut cols[1]);
            let mut disabled = String::from("read-only");
            let _ = Input::new(&mut disabled)
                .label("Disabled")
                .disabled(true)
                .show(&mut cols[2]);
        });
    });
    ui.add_space(12.0);

    Card::new().title("Checks & toggles").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(20.0, 8.0);
            let _ = Checkbox::new(&mut s.agree, "Accept terms").show(ui);
            let _ = Checkbox::new(&mut s.mixed, "Some selected")
                .indeterminate(true)
                .show(ui);
            let mut on = true;
            let _ = Checkbox::new(&mut on, "Disabled").disabled(true).show(ui);
            let _ = Toggle::new(&mut s.notifications)
                .label("Notifications")
                .show(ui);
            let _ = Toggle::new(&mut s.airplane).label("Airplane mode").show(ui);
        });
        ui.add_space(8.0);
        let _ = RadioGroup::new(&mut s.radio, &["Realtime", "Hourly", "Daily"])
            .row(true)
            .show(ui);
        readout(
            ui,
            &t,
            &format!(
                "agree={} notifications={} airplane={} radio={}",
                s.agree, s.notifications, s.airplane, s.radio
            ),
        );
    });
    ui.add_space(12.0);

    Card::new().title("Sliders & segments").show(ui, |ui| {
        let _ = Slider::new(&mut s.volume, 0.0..=100.0)
            .step(5.0)
            .label("Volume")
            .show(ui);
        ui.add_space(6.0);
        let _ = Slider::new(&mut s.threshold, 0.0..=1.0)
            .label("Threshold")
            .show(ui);
        ui.add_space(10.0);
        let _ = ToggleGroup::new(&mut s.segment, &["1h", "24h", "7d", "30d"]).show(ui);
        readout(
            ui,
            &t,
            &format!(
                "volume={} threshold={:.2} segment={}",
                s.volume, s.threshold, s.segment
            ),
        );
    });
}

fn readout(ui: &mut egui::Ui, t: &Theme, text: &str) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(text)
            .font(t.mono(t.type_scale.sm))
            .color(t.fg[2]),
    );
}
