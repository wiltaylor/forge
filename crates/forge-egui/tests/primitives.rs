//! Interaction-contract tests for the primitives, driven headless through
//! egui_kittest (AccessKit queries — no GPU needed).

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use forge_egui::prelude::*;

fn themed_harness<'a>(app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut harness = Harness::new_ui(app);
    Theme::dark().apply(&harness.ctx);
    harness.run();
    harness
}

#[test]
fn button_click_submits() {
    let clicked = std::cell::Cell::new(false);
    let mut harness = themed_harness(|ui| {
        if Button::new("Save")
            .variant(Variant::Primary)
            .show(ui)
            .submitted()
        {
            clicked.set(true);
        }
    });
    harness.get_by_label("Save").click();
    harness.run();
    drop(harness);
    assert!(clicked.get());
}

#[test]
fn disabled_button_does_not_submit() {
    let clicked = std::cell::Cell::new(false);
    let mut harness = themed_harness(|ui| {
        if Button::new("Save").disabled(true).show(ui).submitted() {
            clicked.set(true);
        }
    });
    harness.get_by_label("Save").click();
    harness.run();
    drop(harness);
    assert!(!clicked.get());
}

#[test]
fn icon_button_exposes_accessible_label() {
    let mut harness = themed_harness(|ui| {
        let _ = IconButton::new(Glyph::Gear, "Settings").show(ui);
    });
    harness.run();
    // The accessible name is the label, not the glyph.
    let _ = harness.get_by_label("Settings");
}

#[test]
fn outcome_merge_prefers_significance() {
    use Outcome::*;
    assert_eq!(Ignored.merge(Changed), Changed);
    assert_eq!(Changed.merge(Submitted), Submitted);
    assert_eq!(Consumed.merge(Ignored), Consumed);
    assert!(!Ignored.is_handled());
    assert!(Cancelled.is_handled());
}
