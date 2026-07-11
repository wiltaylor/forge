//! Interaction-contract tests for M4 feedback + overlay widgets, driven
//! headless through egui_kittest (AccessKit queries — no GPU needed).

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
fn modal_escape_closes() {
    let open = RefCell::new(true);
    let mut harness = themed_harness(|ui| {
        let mut is_open = open.borrow_mut();
        let _ = Modal::new("Confirm rollout").show(&ui.ctx().clone(), &mut is_open, |ui| {
            ui.label("Body");
        });
    });
    // Renders while open.
    let _ = harness.get_by_label("Confirm rollout");
    harness.key_press(egui::Key::Escape);
    harness.run();
    drop(harness);
    assert!(!*open.borrow());
}

#[test]
fn modal_close_button_closes() {
    let open = RefCell::new(true);
    let mut harness = themed_harness(|ui| {
        let mut is_open = open.borrow_mut();
        let _ = Modal::new("Settings").show(&ui.ctx().clone(), &mut is_open, |ui| {
            ui.label("Body");
        });
    });
    harness.get_by_label("Close").click();
    harness.run();
    drop(harness);
    assert!(!*open.borrow());
}

#[test]
fn dropdown_menu_opens_and_returns_clicked_index() {
    let items = [
        MenuItem::new("Copy"),
        MenuItem::new("Duplicate"),
        MenuItem::new("Delete").danger(true),
    ];
    let picked = RefCell::new(None::<usize>);
    let mut harness = themed_harness(|ui| {
        let selected = DropdownMenu::new(&items).show(ui, |ui| Button::new("Actions").show(ui));
        if selected.is_some() {
            *picked.borrow_mut() = selected;
        }
    });
    harness.get_by_label("Actions").click();
    harness.run();
    harness.get_by_label("Duplicate").click();
    harness.run();
    drop(harness);
    assert_eq!(*picked.borrow(), Some(1));
}

#[test]
fn alert_renders_title() {
    let mut harness = themed_harness(|ui| {
        let _ = Alert::new(Tone::Warning, "Certificate expiring")
            .message("Renew soon")
            .show(ui);
    });
    harness.run();
    let _ = harness.get_by_label("Certificate expiring");
    let _ = harness.get_by_label("Renew soon");
}

#[test]
fn progress_value_clamps() {
    assert_eq!(Progress::new(1.5).value(), 1.0);
    assert_eq!(Progress::new(-0.25).value(), 0.0);
    assert_eq!(Progress::new(0.42).value(), 0.42);
    assert_eq!(Progress::new(f32::NAN).value(), 0.0);
}
