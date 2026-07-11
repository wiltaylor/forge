//! Pickers: Select, Combobox, and ListBox bound to explicit app-owned state.

use forge_egui::prelude::*;

const REGIONS: &[&str] = &["us-east-1", "us-west-2", "eu-central-1", "ap-southeast-2"];
const LANGS: &[&str] = &[
    "Rust",
    "TypeScript",
    "Python",
    "Go",
    "Zig",
    "Kotlin",
    "Ruby",
];
const SERVICES: &[&str] = &[
    "gateway", "auth", "billing", "search", "ingest", "metrics", "docs",
];

pub struct PickersState {
    select: SelectState,
    combo: ComboboxState,
    single: ListBoxState,
    multi: ListBoxState,
}

impl Default for PickersState {
    fn default() -> PickersState {
        PickersState {
            select: SelectState::default(),
            combo: ComboboxState::default(),
            single: ListBoxState {
                selected: vec![0],
                highlight: 0,
            },
            multi: ListBoxState {
                selected: vec![1, 3],
                highlight: 1,
            },
        }
    }
}

pub fn draw(ui: &mut egui::Ui, s: &mut PickersState) {
    let t = Theme::of(ui.ctx());

    ui.columns(2, |cols| {
        Card::new().title("Select").show(&mut cols[0], |ui| {
            let _ = Select::new(&mut s.select, REGIONS)
                .label("Region")
                .placeholder("Pick a region")
                .show(ui);
            readout(ui, &t, &format!("value={:?}", s.select.value));
        });
        Card::new().title("Combobox").show(&mut cols[1], |ui| {
            let _ = Combobox::new(&mut s.combo, LANGS)
                .label("Language")
                .placeholder("Type to filter…")
                .empty_text("No matches")
                .show(ui);
            readout(ui, &t, &format!("value={:?}", s.combo.value));
        });
    });
    ui.add_space(12.0);

    ui.columns(2, |cols| {
        Card::new()
            .title("ListBox (single)")
            .show(&mut cols[0], |ui| {
                let _ = ListBox::new(&mut s.single, SERVICES).show(ui);
                readout(ui, &t, &format!("selected={:?}", s.single.selected));
            });
        Card::new()
            .title("ListBox (multi)")
            .show(&mut cols[1], |ui| {
                let _ = ListBox::new(&mut s.multi, SERVICES).multiple(true).show(ui);
                readout(ui, &t, &format!("selected={:?}", s.multi.selected));
            });
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
