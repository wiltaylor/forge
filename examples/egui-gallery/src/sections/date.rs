//! Date: inline Calendar + DatePicker with min/max bounds.

use forge_egui::prelude::*;

pub struct DateState {
    pub cal: CalendarState,
    pub picker: DatePickerState,
}

impl Default for DateState {
    fn default() -> DateState {
        DateState {
            cal: CalendarState {
                month: (2026, 7),
                value: Some("2026-07-11".to_owned()),
            },
            picker: DatePickerState {
                open: false,
                cal: CalendarState {
                    month: (2026, 7),
                    value: None,
                },
            },
        }
    }
}

pub fn draw(ui: &mut egui::Ui, s: &mut DateState) {
    let t = Theme::of(ui.ctx());

    ui.columns(2, |cols| {
        Card::new().title("Calendar").show(&mut cols[0], |ui| {
            let _ = Calendar::new(&mut s.cal).show(ui);
            readout(ui, &t, &format!("value={:?}", s.cal.value));
        });
        Card::new()
            .title("DatePicker (min/max)")
            .show(&mut cols[1], |ui| {
                let _ = DatePicker::new(&mut s.picker)
                    .label("Deploy date")
                    .placeholder("Pick a date…")
                    .help("Bounded to July 2026")
                    .min("2026-07-01")
                    .max("2026-07-31")
                    .show(ui);
                readout(
                    ui,
                    &t,
                    &format!("open={} value={:?}", s.picker.open, s.picker.cal.value),
                );
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
