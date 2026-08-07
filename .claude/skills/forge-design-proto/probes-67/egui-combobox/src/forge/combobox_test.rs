//! The combobox Contract, exercised headlessly.

use egui::Key;
use egui_kittest::{kittest::Queryable, Harness};

use super::{
    combobox::{ComboBox, ComboBoxOption, ComboBoxState},
    response::Outcome,
    theme::Theme,
};

fn options() -> Vec<ComboBoxOption<'static>> {
    vec![
        ComboBoxOption::new("us-east-1"),
        ComboBoxOption::new("eu-west-1"),
        ComboBoxOption::new("eu-west-2"),
        ComboBoxOption::new("cn-north-1").disabled(true),
    ]
}

struct Probe {
    state: ComboBoxState,
    outcome: Outcome,
}

fn harness<'a>(options: &'a [ComboBoxOption<'a>]) -> Harness<'a, Probe> {
    Harness::new_ui_state(
        move |ui, probe: &mut Probe| {
            Theme::dark().apply(ui.ctx());
            let result = ComboBox::new("region", &mut probe.state)
                .options(options)
                .placeholder("Select a region")
                .empty_text("No region matches.")
                .width(240.0)
                .show(ui);
            // `Harness::run` steps until the ui settles, so keep the last
            // outcome that was not `Ignored`.
            if result.outcome != Outcome::Ignored {
                probe.outcome = result.outcome;
            }
        },
        Probe {
            state: ComboBoxState::default(),
            outcome: Outcome::Ignored,
        },
    )
}

fn type_text(harness: &mut Harness<'_, Probe>, text: &str) {
    harness
        .input_mut()
        .events
        .push(egui::Event::Text(text.to_owned()));
    harness.run();
}

#[test]
fn opens_with_nothing_selected() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    assert_eq!(harness.state().state.value, None);
    assert!(!harness.state().state.open);
}

#[test]
fn a_click_opens_the_popup() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    assert!(harness.state().state.open);
}

#[test]
fn typing_filters_on_a_substring_of_the_label() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "WEST");
    assert_eq!(harness.state().state.query, "WEST");
    assert!(harness.query_by_label("eu-west-1").is_some());
    assert!(harness.query_by_label("us-east-1").is_none());
}

#[test]
fn nothing_matching_shows_the_empty_line() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "zz");
    assert!(harness.query_by_label("eu-west-1").is_none());
    assert!(harness.state().state.open);
}

#[test]
fn enter_commits_the_active_option_and_clears_the_query() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "eu-west-2");
    harness.key_press(Key::Enter);
    harness.run();

    assert_eq!(harness.state().state.value, Some(2));
    assert!(!harness.state().state.open);
    assert!(harness.state().state.query.is_empty());
    assert_eq!(harness.state().outcome, Outcome::Submitted);
}

#[test]
fn escape_closes_and_keeps_the_committed_value() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "eu-west-2");
    harness.key_press(Key::Enter);
    harness.run();

    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "us");
    harness.key_press(Key::Escape);
    harness.run();

    assert_eq!(harness.state().state.value, Some(2));
    assert!(!harness.state().state.open);
    assert!(harness.state().state.query.is_empty());
}

#[test]
fn a_disabled_option_does_not_commit() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "cn-north-1");
    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();

    assert_eq!(harness.state().state.value, None);
    assert!(harness.state().state.open);
}

#[test]
fn down_moves_the_keyboard_and_stops_at_the_last_option() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    for _ in 0..10 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    // The last option is disabled, so Enter is a no-op and the popup stays.
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().state.open);
    assert_eq!(harness.state().state.value, None);
}

#[test]
fn probe67_snapshot() {
    let options = options();
    let mut harness = harness(&options);
    harness.run();
    harness.get_by_role(egui::accesskit::Role::ComboBox).click();
    harness.run();
    type_text(&mut harness, "eu");
    harness.run();
    let image = harness.render().unwrap();
    image.save("/tmp/claude-1000/-home-wil-orca-forge/c1e71b01-2aa4-4e29-ae5e-42f9abc7a41b/scratchpad/probe-egui-combobox.png").unwrap();
}
