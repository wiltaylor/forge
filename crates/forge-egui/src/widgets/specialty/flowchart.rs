//! Read-only flowchart with automatic layered layout (no deps) — the egui
//! sibling of the web `Flowchart` and forge-tui's box-drawing version. Nodes
//! become rounded rects arranged left→right by longest-path layer; edges
//! route as elbow polylines with arrowheads. Static render: pan/zoom and
//! editing belong to the (later) NodeGraph widget.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::Tone;
use egui::{CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::HashMap;

const NODE_H: f32 = 36.0;
const PAD_X: f32 = 14.0;
const GAP_X: f32 = 56.0;
const GAP_Y: f32 = 20.0;
const ACCENT_BAR_W: f32 = 3.0;

/// A flowchart node. `tone` tints the node's leading edge (Neutral = none).
#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    pub tone: Tone,
}

impl FlowNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> FlowNode {
        FlowNode {
            id: id.into(),
            label: label.into(),
            tone: Tone::Neutral,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// A directed edge. `broken` renders dashed in danger tint (a failing link).
#[derive(Clone, Debug, PartialEq)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub broken: bool,
}

impl FlowEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> FlowEdge {
        FlowEdge {
            from: from.into(),
            to: to.into(),
            label: None,
            broken: false,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn broken(mut self, broken: bool) -> Self {
        self.broken = broken;
        self
    }
}

/// Longest-path layering (cycles break by capping relaxation passes) —
/// the same algorithm as forge-tui's flowchart.
fn layers<'a>(nodes: &'a [FlowNode], edges: &'a [FlowEdge]) -> HashMap<&'a str, usize> {
    let mut layer: HashMap<&str, usize> = nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    for _ in 0..nodes.len().max(1) {
        let mut changed = false;
        for e in edges {
            let (Some(&lf), Some(&lt)) = (layer.get(e.from.as_str()), layer.get(e.to.as_str()))
            else {
                continue;
            };
            if lt < lf + 1 {
                layer.insert(e.to.as_str(), lf + 1);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    layer
}

/// Compute node rects (origin 0,0; parallel to `nodes`) and the total size.
/// Deterministic: columns follow layer order, rows follow `nodes` slice order.
pub(crate) fn layout(
    nodes: &[FlowNode],
    edges: &[FlowEdge],
    label_w: impl Fn(&str) -> f32,
) -> (Vec<Rect>, Vec2) {
    if nodes.is_empty() {
        return (Vec::new(), Vec2::ZERO);
    }
    let layer = layers(nodes, edges);
    let n_layers = layer.values().max().copied().unwrap_or(0) + 1;

    // Column membership in slice order; column width = widest node.
    let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n_layers];
    for (i, node) in nodes.iter().enumerate() {
        cols[layer[node.id.as_str()]].push(i);
    }
    let node_w = |i: usize| label_w(&nodes[i].label) + PAD_X * 2.0;
    let col_w: Vec<f32> = cols
        .iter()
        .map(|c| {
            c.iter().map(|&i| node_w(i)).fold(NODE_H, f32::max) // at least square-ish
        })
        .collect();
    let col_h = |c: &Vec<usize>| {
        if c.is_empty() {
            0.0
        } else {
            c.len() as f32 * NODE_H + (c.len() - 1) as f32 * GAP_Y
        }
    };
    let max_h = cols.iter().map(col_h).fold(NODE_H, f32::max);

    let mut rects = vec![Rect::NOTHING; nodes.len()];
    let mut x = 0.0;
    for (li, col) in cols.iter().enumerate() {
        let mut y = (max_h - col_h(col)) / 2.0;
        for &i in col {
            let w = node_w(i);
            // Center each node within its column.
            let nx = x + (col_w[li] - w) / 2.0;
            rects[i] = Rect::from_min_size(Pos2::new(nx, y), Vec2::new(w, NODE_H));
            y += NODE_H + GAP_Y;
        }
        x += col_w[li] + GAP_X;
    }
    let width = x - GAP_X;
    (rects, Vec2::new(width, max_h))
}

/// Auto-laid-out static flowchart: `Flowchart::new(&nodes, &edges).show(ui)`.
#[derive(Clone, Debug)]
pub struct Flowchart<'a> {
    nodes: &'a [FlowNode],
    edges: &'a [FlowEdge],
}

impl<'a> Flowchart<'a> {
    pub fn new(nodes: &'a [FlowNode], edges: &'a [FlowEdge]) -> Flowchart<'a> {
        Flowchart { nodes, edges }
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        let label_font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs);

        let (rects, size) = {
            let painter = ui.painter();
            let font = font.clone();
            layout(self.nodes, self.edges, |s| {
                painter
                    .layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        };
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) || self.nodes.is_empty() {
            return ForgeResponse::new(response, Outcome::Ignored);
        }
        let origin = rect.min.to_vec2();
        let painter = ui.painter();
        let index: HashMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();

        // Edges first (nodes overpaint them cleanly).
        for e in self.edges {
            let (Some(&fi), Some(&ti)) = (index.get(e.from.as_str()), index.get(e.to.as_str()))
            else {
                continue;
            };
            let from = rects[fi].translate(origin);
            let to = rects[ti].translate(origin);
            let (x0, y0) = (from.right(), from.center().y);
            let (x1, y1) = (to.left(), to.center().y);
            if x1 <= x0 {
                continue; // only forward edges are routed in v1
            }
            let mid = (x0 + x1) / 2.0;
            let color = if e.broken {
                t.danger.base
            } else {
                t.border.strong
            };
            let stroke = Stroke::new(1.5, color);
            let head = 7.0;
            let points = vec![
                Pos2::new(x0, y0),
                Pos2::new(mid, y0),
                Pos2::new(mid, y1),
                Pos2::new(x1 - head, y1),
            ];
            if e.broken {
                painter.add(egui::Shape::dashed_line(&points, stroke, 5.0, 4.0));
            } else {
                painter.add(egui::Shape::line(points, stroke));
            }
            // Arrowhead.
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(x1, y1),
                    Pos2::new(x1 - head, y1 - 4.5),
                    Pos2::new(x1 - head, y1 + 4.5),
                ],
                if e.broken {
                    t.danger.base
                } else {
                    t.accent.base
                },
                Stroke::NONE,
            ));
            // Edge label — a small chip at the elbow midpoint.
            if let Some(label) = &e.label {
                let g = painter.layout_no_wrap(label.clone(), label_font.clone(), t.fg[2]);
                let at = Pos2::new(mid, (y0 + y1) / 2.0);
                let chip = Rect::from_center_size(at, g.size() + Vec2::new(8.0, 4.0));
                painter.rect_filled(chip, CornerRadius::same(t.radius.sm as u8), t.bg[0]);
                painter.galley(chip.min + Vec2::new(4.0, 2.0), g, t.fg[2]);
            }
        }

        // Nodes.
        for (i, node) in self.nodes.iter().enumerate() {
            let r = rects[i].translate(origin);
            let radius = CornerRadius::same(t.radius.md as u8);
            painter.rect_filled(r, radius, t.bg[2]);
            painter.rect_stroke(
                r,
                radius,
                Stroke::new(1.0, t.border.default),
                StrokeKind::Inside,
            );
            if node.tone != Tone::Neutral {
                let (base, _, _) = node.tone.triple(&t);
                let bar = Rect::from_min_max(
                    r.min + Vec2::new(0.0, 3.0),
                    Pos2::new(r.min.x + ACCENT_BAR_W, r.max.y - 3.0),
                );
                painter.rect_filled(bar, CornerRadius::same(1), base);
            }
            let g = painter.layout_no_wrap(node.label.clone(), font.clone(), t.fg[0]);
            painter.galley(r.center() - g.size() / 2.0, g, t.fg[0]);
        }

        ForgeResponse::new(response, Outcome::Ignored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> (Vec<FlowNode>, Vec<FlowEdge>) {
        let nodes = vec![
            FlowNode::new("src", "Checkout"),
            FlowNode::new("lint", "Lint"),
            FlowNode::new("build", "Build").tone(Tone::Accent),
            FlowNode::new("test", "Test"),
            FlowNode::new("pkg", "Package"),
            FlowNode::new("deploy", "Deploy").tone(Tone::Success),
        ];
        let edges = vec![
            FlowEdge::new("src", "lint"),
            FlowEdge::new("src", "build"),
            FlowEdge::new("build", "test"),
            FlowEdge::new("test", "pkg").label("on green"),
            FlowEdge::new("pkg", "deploy").broken(true),
        ];
        (nodes, edges)
    }

    fn measure(s: &str) -> f32 {
        s.len() as f32 * 7.0
    }

    #[test]
    fn layout_is_deterministic() {
        let (nodes, edges) = demo();
        let (a, size_a) = layout(&nodes, &edges, measure);
        let (b, size_b) = layout(&nodes, &edges, measure);
        assert_eq!(a, b);
        assert_eq!(size_a, size_b);
    }

    #[test]
    fn layout_has_no_overlapping_nodes() {
        let (nodes, edges) = demo();
        let (rects, size) = layout(&nodes, &edges, measure);
        assert_eq!(rects.len(), nodes.len());
        for (i, a) in rects.iter().enumerate() {
            assert!(a.width() > 0.0 && a.height() > 0.0);
            assert!(a.max.x <= size.x + 0.5 && a.max.y <= size.y + 0.5);
            for b in rects.iter().skip(i + 1) {
                assert!(
                    !a.expand(1.0).intersects(*b),
                    "nodes {i} and another overlap: {a:?} vs {b:?}"
                );
            }
        }
    }

    #[test]
    fn layout_layers_follow_edges() {
        let (nodes, edges) = demo();
        let (rects, _) = layout(&nodes, &edges, measure);
        let idx = |id: &str| nodes.iter().position(|n| n.id == id).unwrap();
        // Every forward edge should point left→right.
        for e in &edges {
            assert!(
                rects[idx(&e.from)].right() < rects[idx(&e.to)].left(),
                "edge {}→{} not left-to-right",
                e.from,
                e.to
            );
        }
    }
}
