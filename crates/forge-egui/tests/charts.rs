//! M7 tests: nice-tick axis math plus kittest interaction contracts for the
//! calendar widgets (headless AccessKit — no GPU needed).

use forge_egui::prelude::*;

fn assert_ticks(got: Vec<f64>, want: &[f64]) {
    assert_eq!(got.len(), want.len(), "got {got:?}, want {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-9, "got {got:?}, want {want:?}");
    }
}

#[test]
fn nice_ticks_round_ranges() {
    assert_ticks(
        nice_ticks(0.0, 100.0, 5),
        &[0.0, 20.0, 40.0, 60.0, 80.0, 100.0],
    );
    // The web algorithm's default target (4): step 50 is the first nice step.
    assert_ticks(nice_ticks(0.0, 100.0, 4), &[0.0, 50.0, 100.0]);
}

#[test]
fn nice_ticks_awkward_max_is_sensible() {
    // 0..7 → step 2, last tick covers the max.
    assert_ticks(nice_ticks(0.0, 7.0, 4), &[0.0, 2.0, 4.0, 6.0, 8.0]);
    let ticks = nice_ticks(0.0, 97.0, 4);
    assert_eq!(ticks.first(), Some(&0.0));
    assert!(*ticks.last().unwrap() >= 97.0);
}

#[test]
fn nice_ticks_negative_ranges() {
    assert_ticks(nice_ticks(-50.0, 100.0, 4), &[-50.0, 0.0, 50.0, 100.0]);
    assert_ticks(nice_ticks(-8.0, -1.0, 4), &[-8.0, -6.0, -4.0, -2.0, 0.0]);
}

#[test]
fn nice_ticks_degenerate_input_never_panics() {
    assert_eq!(nice_ticks(0.0, 0.0, 4), vec![0.0, 1.0]);
    assert_eq!(nice_ticks(5.0, 5.0, 4).len(), 2);
    assert_eq!(nice_ticks(3.0, 1.0, 4).len(), 2);
}

#[cfg(feature = "calendar")]
mod calendar {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use forge_egui::prelude::*;
    use std::cell::RefCell;

    fn themed_harness<'a>(app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
        let mut harness = Harness::new_ui(app);
        Theme::dark().apply(&harness.ctx);
        harness.run();
        harness
    }

    #[test]
    fn calendar_click_selects_day() {
        let state = RefCell::new(CalendarState {
            month: (2026, 7),
            value: None,
        });
        let changed = RefCell::new(false);
        let mut harness = themed_harness(|ui| {
            let mut s = state.borrow_mut();
            if Calendar::new(&mut s).show(ui).changed() {
                *changed.borrow_mut() = true;
            }
        });
        // Day cells are labeled with their full ISO date.
        harness.get_by_label("2026-07-15").click();
        harness.run();
        drop(harness);
        assert_eq!(state.borrow().value.as_deref(), Some("2026-07-15"));
        assert!(*changed.borrow());
    }

    #[test]
    fn calendar_min_max_disable_out_of_range_days() {
        let state = RefCell::new(CalendarState {
            month: (2026, 7),
            value: None,
        });
        let mut harness = themed_harness(|ui| {
            let mut s = state.borrow_mut();
            let _ = Calendar::new(&mut s)
                .min("2026-07-05")
                .max("2026-07-20")
                .show(ui);
        });
        harness.get_by_label("2026-07-04").click();
        harness.run();
        assert_eq!(state.borrow().value, None); // below min — ignored
        harness.get_by_label("2026-07-05").click();
        harness.run();
        drop(harness);
        assert_eq!(state.borrow().value.as_deref(), Some("2026-07-05"));
    }

    #[test]
    fn calendar_nav_shifts_month() {
        let state = RefCell::new(CalendarState {
            month: (2026, 1),
            value: None,
        });
        let mut harness = themed_harness(|ui| {
            let mut s = state.borrow_mut();
            let _ = Calendar::new(&mut s).show(ui);
        });
        harness.get_by_label("Previous month").click();
        harness.run();
        drop(harness);
        assert_eq!(state.borrow().month, (2025, 12));
    }

    #[test]
    fn date_picker_opens_and_picks() {
        let state = RefCell::new(DatePickerState {
            open: false,
            cal: CalendarState {
                month: (2026, 7),
                value: None,
            },
        });
        let mut harness = themed_harness(|ui| {
            let mut s = state.borrow_mut();
            let _ = DatePicker::new(&mut s).label("Date").show(ui);
        });
        harness
            .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Date")
            .click();
        harness.run();
        assert!(state.borrow().open);
        harness.get_by_label("2026-07-11").click();
        harness.run();
        drop(harness);
        assert_eq!(state.borrow().cal.value.as_deref(), Some("2026-07-11"));
        assert!(!state.borrow().open); // closes on pick
    }
}
