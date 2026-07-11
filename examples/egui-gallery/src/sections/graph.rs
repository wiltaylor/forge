//! Graph: the NodeGraph editor. The widget only *requests* mutations — this
//! section applies each returned `GraphEvent` to its own nodes/edges.

use forge_egui::prelude::*;
use forge_egui::widgets::{GraphEdge, GraphEvent, GraphNode, NodeGraph, NodeGraphState, PortType};

pub struct GraphState {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub state: NodeGraphState,
    last: Option<String>,
}

impl Default for GraphState {
    fn default() -> Self {
        let nodes = vec![
            GraphNode::new("hook", "Webhook", egui::pos2(30.0, 110.0))
                .tone(Tone::Info)
                .output("fire", PortType::Trigger)
                .output("payload", PortType::Object)
                .output("route", PortType::String),
            GraphNode::new("xform", "Transform", egui::pos2(260.0, 55.0))
                .tone(Tone::Accent)
                .input("run", PortType::Trigger)
                .input("data", PortType::Object)
                .output("done", PortType::Trigger)
                .output("result", PortType::Object)
                .output("count", PortType::Number),
            GraphNode::new("branch", "Branch", egui::pos2(480.0, 140.0))
                .tone(Tone::Warning)
                .input("run", PortType::Trigger)
                .input("value", PortType::Number)
                .input("invert", PortType::Boolean)
                .output("high", PortType::Trigger)
                .output("low", PortType::Trigger),
            GraphNode::new("out", "Output", egui::pos2(700.0, 95.0))
                .tone(Tone::Success)
                .input("run", PortType::Trigger)
                .input("payload", PortType::Any),
        ];
        let edges = vec![
            GraphEdge::new("hook", 0, "xform", 0),
            GraphEdge::new("hook", 1, "xform", 1),
            GraphEdge::new("xform", 0, "branch", 0),
            GraphEdge::new("xform", 2, "branch", 1),
            GraphEdge::new("branch", 0, "out", 0),
            GraphEdge::new("xform", 1, "out", 1),
        ];
        GraphState {
            nodes,
            edges,
            state: NodeGraphState::default(),
            last: None,
        }
    }
}

/// Apply the requested events to the caller-owned graph.
fn apply(state: &mut GraphState, events: Vec<GraphEvent>) {
    for event in events {
        match &event {
            GraphEvent::Moved { node, pos } => {
                if let Some(n) = state.nodes.iter_mut().find(|n| &n.id == node) {
                    n.pos = *pos;
                }
            }
            GraphEvent::Connected { edge } => {
                // One connection per input: replace anything already wired in.
                state.edges.retain(|e| e.to != edge.to);
                state.edges.push(edge.clone());
            }
            GraphEvent::Disconnected { index } => {
                if *index < state.edges.len() {
                    state.edges.remove(*index);
                }
            }
            GraphEvent::Selected(_) => {}
            GraphEvent::Deleted(id) => {
                state.nodes.retain(|n| &n.id != id);
                state.edges.retain(|e| &e.from.0 != id && &e.to.0 != id);
            }
        }
        // Readout (drags fire per frame — collapse to something readable).
        state.last = Some(match event {
            GraphEvent::Moved { node, pos } => {
                format!("moved {node} → ({:.0}, {:.0})", pos.x, pos.y)
            }
            GraphEvent::Connected { edge } => format!(
                "connected {}:{} → {}:{}",
                edge.from.0, edge.from.1, edge.to.0, edge.to.1
            ),
            GraphEvent::Disconnected { index } => format!("disconnected edge #{index}"),
            GraphEvent::Selected(sel) => match sel {
                Some(id) => format!("selected {id}"),
                None => "deselected".to_owned(),
            },
            GraphEvent::Deleted(id) => format!("deleted {id}"),
        });
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut GraphState) {
    let t = Theme::of(ui.ctx());

    Card::new()
        .title("Node graph — drag titles, connect ports, Delete removes")
        .show(ui, |ui| {
            let events = NodeGraph::new(&mut state.state, &state.nodes, &state.edges)
                .height(430.0)
                .show(ui);
            apply(state, events);

            ui.add_space(8.0);
            let readout = state
                .last
                .clone()
                .unwrap_or_else(|| "drag from an output dot to a matching input".to_owned());
            ui.label(
                egui::RichText::new(readout)
                    .font(t.mono(t.type_scale.sm))
                    .color(t.fg[2]),
            );
            ui.label(
                egui::RichText::new(
                    "pan: drag canvas / middle-drag · zoom: scroll · connect: drag output → \
                     input (types must match; Any is a wildcard, Trigger only pairs with \
                     Trigger) · click edge midpoint then × or Delete to disconnect",
                )
                .font(t.font(
                    ui.ctx(),
                    forge_egui::theme::FontWeight::Regular,
                    t.type_scale.xs,
                ))
                .color(t.fg[2]),
            );
        });
}
