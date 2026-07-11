//! Interaction-contract tests for the NodeGraph editor, driven headless
//! through egui_kittest. The pure math (compatibility matrix, pan/zoom
//! round-trip, elbow/midpoint geometry) lives in the widget's in-file tests.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use forge_egui::prelude::*;
use std::cell::RefCell;

fn demo_nodes() -> Vec<GraphNode> {
    vec![
        GraphNode::new("a", "Alpha", egui::pos2(60.0, 60.0))
            .output("fire", PortType::Trigger)
            .output("payload", PortType::Object),
        GraphNode::new("b", "Beta", egui::pos2(340.0, 90.0))
            .input("run", PortType::Trigger)
            .input("data", PortType::Object),
    ]
}

#[test]
fn node_click_selects_and_canvas_click_deselects() {
    let state = RefCell::new(NodeGraphState::default());
    let events = RefCell::new(Vec::<GraphEvent>::new());
    let nodes = demo_nodes();
    let mut harness = Harness::new_ui(|ui| {
        let evs = NodeGraph::new(&mut state.borrow_mut(), &nodes, &[]).show(ui);
        events.borrow_mut().extend(evs);
    });
    Theme::dark().apply(&harness.ctx);
    harness.run();

    harness.get_by_label("Alpha").click();
    harness.run();
    assert_eq!(state.borrow().selected.as_deref(), Some("a"));
    assert!(events
        .borrow()
        .contains(&GraphEvent::Selected(Some("a".into()))));

    // Click empty canvas well away from both nodes → deselect.
    events.borrow_mut().clear();
    let empty = egui::pos2(60.0, 400.0);
    harness.drag_at(empty);
    harness.step();
    harness.drop_at(empty);
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().selected, None);
    assert!(events.borrow().contains(&GraphEvent::Selected(None)));
}

#[test]
fn edges_render_without_panic_and_stale_selection_clears() {
    let mut initial = NodeGraphState::default();
    // Points past the end of the edges slice — must be dropped, not panic.
    initial.selected_edge = Some(5);
    initial.selected = Some("gone".into());
    let state = RefCell::new(initial);
    let nodes = demo_nodes();
    let edges = vec![
        GraphEdge::new("a", 0, "b", 0),
        GraphEdge::new("a", 1, "b", 1),
    ];
    let mut harness = Harness::new_ui(|ui| {
        let _ = NodeGraph::new(&mut state.borrow_mut(), &nodes, &edges).show(ui);
    });
    Theme::dark().apply(&harness.ctx);
    harness.run();
    drop(harness);
    assert_eq!(state.borrow().selected_edge, None);
    assert_eq!(state.borrow().selected, None, "missing node id clears");
}
