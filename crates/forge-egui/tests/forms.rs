//! Interaction-contract tests for the form widgets, driven headless through
//! egui_kittest (AccessKit queries — no GPU needed).

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
fn checkbox_click_toggles() {
    let checked = RefCell::new(false);
    let mut harness = themed_harness(|ui| {
        let mut value = checked.borrow_mut();
        let _ = Checkbox::new(&mut value, "Accept terms").show(ui);
    });
    harness.get_by_label("Accept terms").click();
    harness.run();
    drop(harness);
    assert!(*checked.borrow());
}

#[test]
fn toggle_click_flips() {
    let on = RefCell::new(false);
    let mut harness = themed_harness(|ui| {
        let mut value = on.borrow_mut();
        let _ = Toggle::new(&mut value).label("Notifications").show(ui);
    });
    harness.get_by_label("Notifications").click();
    harness.run();
    drop(harness);
    assert!(*on.borrow());
}

#[test]
fn input_typing_updates_bound_string() {
    let text = RefCell::new(String::new());
    let mut harness = themed_harness(|ui| {
        let mut value = text.borrow_mut();
        let _ = Input::new(&mut value).placeholder("Name").show(ui);
    });
    let node = harness.get_by_role(egui::accesskit::Role::TextInput);
    node.focus();
    node.type_text("hello");
    harness.run();
    drop(harness);
    assert_eq!(*text.borrow(), "hello");
}

#[test]
fn select_opens_and_commits_option() {
    let state = RefCell::new(SelectState::default());
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = Select::new(&mut s, &["Alpha", "Beta", "Gamma"])
            .label("Env")
            .show(ui);
    });
    // The label row is its own AccessKit label node; select the field by role.
    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Env")
        .click();
    harness.run();
    assert!(state.borrow().open);
    harness.get_by_label("Beta").click();
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().value, Some(1));
    assert!(!state.borrow().open);
}

#[test]
fn slider_arrow_keys_change_value_when_focused() {
    let value = RefCell::new(50.0f64);
    let mut harness = themed_harness(|ui| {
        let mut v = value.borrow_mut();
        let _ = Slider::new(&mut v, 0.0..=100.0)
            .step(5.0)
            .label("Volume")
            .show(ui);
    });
    harness
        .get_by_role_and_label(egui::accesskit::Role::Slider, "Volume")
        .focus();
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    harness.key_press(egui::Key::ArrowLeft);
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    drop(harness);
    assert_eq!(*value.borrow(), 45.0);
}

#[test]
fn list_box_multi_click_toggles() {
    let state = RefCell::new(ListBoxState::default());
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = ListBox::new(&mut s, &["One", "Two", "Three"])
            .multiple(true)
            .show(ui);
    });
    harness.get_by_label("Two").click();
    harness.run();
    assert_eq!(state.borrow().selected, vec![1]);
    harness.get_by_label("Three").click();
    harness.run();
    assert_eq!(state.borrow().selected, vec![1, 2]);
    harness.get_by_label("Two").click();
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().selected, vec![2]);
}
