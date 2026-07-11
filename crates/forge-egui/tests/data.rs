//! Interaction-contract tests for the data widgets, driven headless through
//! egui_kittest (AccessKit queries — no GPU needed).
//!
//! Kanban drag-and-drop is not exercised here: kittest has no pointer-drag
//! simulation that survives egui's dnd payload plumbing, so the drop-position
//! math is unit-tested inside `widgets/data/kanban.rs` instead.

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

// ---------------------------------------------------------------- Table

fn demo_columns() -> [Column<'static>; 3] {
    [
        Column::new("Name"),
        Column::new("Region"),
        Column::new("CPU").align(egui::Align::Max),
    ]
}

#[test]
fn table_header_click_cycles_sort_asc_desc_none() {
    let state = RefCell::new(TableState::default());
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let cols = demo_columns();
        let _ = Table::new(&mut s, &cols).show(ui, 3, |row| {
            row.text(format!("svc-{}", row.index()));
            row.text("eu-1");
            row.text("12%");
        });
    });
    assert_eq!(state.borrow().sort, None);
    harness.get_by_label("Region").click();
    harness.run();
    assert_eq!(state.borrow().sort, Some((1, SortDir::Asc)));
    harness.get_by_label("Region").click();
    harness.run();
    assert_eq!(state.borrow().sort, Some((1, SortDir::Desc)));
    harness.get_by_label("Region").click();
    harness.run();
    assert_eq!(state.borrow().sort, None);
    // A different column starts back at ascending.
    harness.get_by_label("Name").click();
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().sort, Some((0, SortDir::Asc)));
}

#[test]
fn table_row_click_selects() {
    let state = RefCell::new(TableState::default());
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let cols = demo_columns();
        let _ = Table::new(&mut s, &cols).striped(true).show(ui, 3, |row| {
            row.text(format!("svc-{}", row.index()));
            row.text("eu-1");
            row.text("12%");
        });
    });
    assert_eq!(state.borrow().selected, None);
    harness.get_by_label("svc-1").click();
    harness.run();
    assert_eq!(state.borrow().selected, Some(1));
    harness.get_by_label("svc-2").click();
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().selected, Some(2));
}

// ---------------------------------------------------------------- Tree

fn demo_tree() -> Vec<TreeNode> {
    vec![TreeNode::new("src", "src")
        .child(
            TreeNode::new("src/widgets", "widgets")
                .child(TreeNode::new("src/widgets/table.rs", "table.rs")),
        )
        .child(TreeNode::new("src/lib.rs", "lib.rs"))]
}

#[test]
fn tree_disclosure_click_expands_and_collapses() {
    let state = RefCell::new(TreeState::default());
    let roots = demo_tree();
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = Tree::new(&mut s, &roots).show(ui);
    });
    assert!(!state.borrow().expanded.contains("src"));
    harness.get_by_label("toggle src").click();
    harness.run();
    assert!(state.borrow().expanded.contains("src"));
    // Children became visible.
    harness.get_by_label("widgets");
    harness.get_by_label("toggle src").click();
    harness.run();
    drop(harness);
    assert!(!state.borrow().expanded.contains("src"));
    // Disclosure clicks never select.
    assert_eq!(state.borrow().selected, None);
}

#[test]
fn tree_row_click_selects_by_id() {
    let state = RefCell::new(TreeState::default());
    let roots = demo_tree();
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = Tree::new(&mut s, &roots).show(ui);
    });
    harness.get_by_label("src").click();
    harness.run();
    assert_eq!(state.borrow().selected.as_deref(), Some("src"));
    // Expand, then select a nested row.
    harness.get_by_label("toggle src").click();
    harness.run();
    harness.get_by_label("lib.rs").click();
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().selected.as_deref(), Some("src/lib.rs"));
}

// ---------------------------------------------------------------- Accordion / Collapsible

#[test]
fn accordion_opens_one_panel_at_a_time() {
    let state = RefCell::new(AccordionState::default());
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = Accordion::new(&mut s, &["First", "Second", "Third"]).show(ui, |i, ui| {
            ui.label(format!("body {i}"));
        });
    });
    assert_eq!(state.borrow().open, None);
    harness.get_by_label("First").click();
    harness.run();
    assert_eq!(state.borrow().open, Some(0));
    harness.get_by_label("Second").click();
    harness.run();
    assert_eq!(state.borrow().open, Some(1));
    // Clicking the open panel closes it.
    harness.get_by_label("Second").click();
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().open, None);
}

#[test]
fn collapsible_header_click_toggles_body() {
    let shown = RefCell::new(false);
    let mut harness = themed_harness(|ui| {
        *shown.borrow_mut() = false;
        let _ = Collapsible::new("Details").show(ui, |ui| {
            *shown.borrow_mut() = true;
            ui.label("hidden body");
        });
    });
    assert!(!*shown.borrow());
    harness.get_by_label("Details").click();
    harness.run();
    assert!(*shown.borrow());
    harness.get_by_label("Details").click();
    harness.run();
    drop(harness);
    assert!(!*shown.borrow());
}

// ---------------------------------------------------------------- JsonViewer

#[test]
fn json_viewer_state_expansion_math() {
    // Pure state-level: pointer-path toggling, root open by default.
    let mut state = JsonViewerState::default();
    assert!(state.is_expanded(""));
    assert!(state.toggle("/nodes"));
    assert!(state.toggle("/nodes/0"));
    assert!(state.is_expanded("/nodes/0"));
    assert!(!state.toggle("/nodes"));
    // Collapsing a parent leaves the child's entry alone (re-expanding the
    // parent restores the subtree exactly).
    assert!(state.is_expanded("/nodes/0"));
}

#[test]
fn json_viewer_click_toggles_object_row() {
    let state = RefCell::new(JsonViewerState::default());
    let value = serde_json::json!({
        "name": "forge",
        "nodes": [{"id": 1}, {"id": 2}],
        "ok": true,
    });
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = JsonViewer::new(&mut s, &value).show(ui);
    });
    // Rows are labeled by pointer path; the array starts collapsed.
    assert!(!state.borrow().is_expanded("/nodes"));
    harness.get_by_label("/nodes").click();
    harness.run();
    assert!(state.borrow().is_expanded("/nodes"));
    harness.get_by_label("/nodes").click();
    harness.run();
    drop(harness);
    assert!(!state.borrow().is_expanded("/nodes"));
}

// ---------------------------------------------------------------- Logs

#[test]
fn logs_follow_chip_toggles_pinned() {
    let state = RefCell::new(LogsState::default());
    let lines: Vec<LogLine> = (0..30)
        .map(|i| {
            LogLine::new(
                format!("12:00:{i:02}"),
                if i % 7 == 0 {
                    Level::Error
                } else {
                    Level::Info
                },
                format!("event {i}"),
            )
        })
        .collect();
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = Logs::new(&mut s, &lines).height(120.0).show(ui);
    });
    assert!(state.borrow().pinned);
    harness.get_by_label("Follow").click();
    harness.run();
    assert!(!state.borrow().pinned);
    harness.get_by_label("Follow").click();
    harness.run();
    drop(harness);
    assert!(state.borrow().pinned);
}

#[test]
fn logs_filter_typing_updates_state() {
    let state = RefCell::new(LogsState::default());
    let lines = vec![
        LogLine::new("12:00:00", Level::Info, "server started"),
        LogLine::new("12:00:01", Level::Warn, "disk pressure"),
    ];
    let mut harness = themed_harness(|ui| {
        let mut s = state.borrow_mut();
        let _ = Logs::new(&mut s, &lines).height(120.0).show(ui);
    });
    let node = harness.get_by_role(egui::accesskit::Role::TextInput);
    node.focus();
    node.type_text("disk");
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().filter, "disk");
}
