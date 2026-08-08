//! Read-only box-drawing flowchart with automatic layered layout (core, no
//! deps) — the terminal sibling of the web `Flowchart`. Nodes become
//! bordered boxes arranged left→right by longest-path layer; edges route as
//! elbow lines through the gaps.

use crate::text;
use crate::theme::{Surface, TextRole};
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Widget};
use std::collections::HashMap;

const GAP: u16 = 6;
const NODE_H: u16 = 3;

#[derive(Clone, Copy, Debug)]
pub struct FlowNode<'a> {
    pub id: &'a str,
    pub label: &'a str,
}

impl<'a> FlowNode<'a> {
    pub fn new(id: &'a str, label: &'a str) -> FlowNode<'a> {
        FlowNode { id, label }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlowEdge<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub label: Option<&'a str>,
}

impl<'a> FlowEdge<'a> {
    pub fn new(from: &'a str, to: &'a str) -> FlowEdge<'a> {
        FlowEdge {
            from,
            to,
            label: None,
        }
    }

    /// Label painted along the edge's horizontal run.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Flowchart<'a> {
    nodes: &'a [FlowNode<'a>],
    edges: &'a [FlowEdge<'a>],
}

impl<'a> Flowchart<'a> {
    pub fn new(nodes: &'a [FlowNode<'a>], edges: &'a [FlowEdge<'a>]) -> Flowchart<'a> {
        Flowchart { nodes, edges }
    }

    /// Rows the chart needs: the tallest layer column at one node per
    /// `NODE_H + 1` rows (for block-editor measurement passes).
    pub fn required_height(&self) -> u16 {
        let layers = self.layers();
        let n_layers = layers.values().max().copied().unwrap_or(0) + 1;
        let mut counts = vec![0u16; n_layers];
        for node in self.nodes {
            counts[layers[node.id]] += 1;
        }
        let tallest = counts.iter().max().copied().unwrap_or(1).max(1);
        tallest * (NODE_H + 1) - 1
    }

    /// Longest-path layering (cycles break arbitrarily by capping passes).
    fn layers(&self) -> HashMap<&'a str, usize> {
        let mut layer: HashMap<&str, usize> = self.nodes.iter().map(|n| (n.id, 0)).collect();
        for _ in 0..self.nodes.len().max(1) {
            let mut changed = false;
            for e in self.edges {
                let (Some(&lf), Some(&lt)) = (layer.get(e.from), layer.get(e.to)) else {
                    continue;
                };
                if lt < lf + 1 {
                    layer.insert(e.to, lf + 1);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        layer
    }
}

impl Widget for Flowchart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        paint(area, |t| {
            if self.nodes.is_empty() {
                return;
            }
            let layers = self.layers();
            let n_layers = layers.values().max().copied().unwrap_or(0) + 1;

            // Column geometry: each layer column is as wide as its widest node.
            let mut col_nodes: Vec<Vec<&FlowNode>> = vec![Vec::new(); n_layers];
            for node in self.nodes {
                col_nodes[layers[node.id]].push(node);
            }
            let col_w: Vec<u16> = col_nodes
                .iter()
                .map(|nodes| {
                    nodes
                        .iter()
                        .map(|n| text::width(n.label) as u16 + 4)
                        .max()
                        .unwrap_or(6)
                })
                .collect();
            // Column gap grows to fit the widest edge label.
            let gap = self
                .edges
                .iter()
                .filter_map(|e| e.label)
                .map(|l| text::width(l) as u16 + 4)
                .max()
                .unwrap_or(0)
                .max(GAP);

            // Place nodes; remember rects by id.
            let mut rects: HashMap<&str, Rect> = HashMap::new();
            let mut x = area.x;
            for (li, nodes) in col_nodes.iter().enumerate() {
                let mut y = area.y;
                for node in nodes {
                    let w = (text::width(node.label) as u16 + 4).min(col_w[li]);
                    if y + NODE_H <= area.y + area.height && x + w <= area.x + area.width {
                        rects.insert(node.id, Rect::new(x, y, w, NODE_H));
                    }
                    y += NODE_H + 1;
                }
                x += col_w[li] + gap;
            }

            // Edges first (nodes overpaint their borders cleanly).
            let edge_style = Style::new().fg(t.border.strong);
            for e in self.edges {
                let (Some(from), Some(to)) = (rects.get(e.from), rects.get(e.to)) else {
                    continue;
                };
                let x0 = from.x + from.width;
                let y0 = from.y + from.height / 2;
                let x1 = to.x;
                let y1 = to.y + to.height / 2;
                if x1 <= x0 {
                    continue; // only forward edges are routed in v1
                }
                let mid = x0 + (x1 - x0) / 2;
                for xx in x0..mid {
                    buf.set_string(xx, y0, "─", edge_style);
                }
                if y0 != y1 {
                    let (top, bot) = (y0.min(y1), y0.max(y1));
                    buf.set_string(mid, y0, if y1 > y0 { "╮" } else { "╯" }, edge_style);
                    for yy in top + 1..bot {
                        buf.set_string(mid, yy, "│", edge_style);
                    }
                    buf.set_string(mid, y1, if y1 > y0 { "╰" } else { "╭" }, edge_style);
                } else {
                    buf.set_string(mid, y0, "─", edge_style);
                }
                for xx in mid + 1..x1.saturating_sub(1) {
                    buf.set_string(xx, y1, "─", edge_style);
                }
                if x1 > area.x {
                    buf.set_string(x1 - 1, y1, "▸", Style::new().fg(t.accent.base));
                }
                if let Some(label) = e.label {
                    // Center the label across the horizontal channel (same-row
                    // edges get the full span; elbowed ones the second run).
                    let (lo, hi) = if y0 == y1 { (x0, x1) } else { (mid, x1) };
                    let avail = hi.saturating_sub(1).saturating_sub(lo + 1) as usize;
                    if avail >= 2 {
                        let lw = text::width(label).min(avail) as u16;
                        let start = lo + 1 + (avail as u16 - lw) / 2;
                        buf.set_string(
                            start,
                            y1,
                            text::truncate(label, avail),
                            Style::new().fg(t.text(TextRole::Tertiary)),
                        );
                    }
                }
            }

            // Nodes.
            for node in self.nodes {
                let Some(rect) = rects.get(node.id) else {
                    continue;
                };
                let block = Block::bordered()
                    .border_style(
                        Style::new()
                            .fg(t.border.default)
                            .bg(t.surface(Surface::Card)),
                    )
                    .style(Style::new().bg(t.surface(Surface::Card)));
                let inner = block.inner(*rect);
                block.render(*rect, buf);
                buf.set_string(
                    inner.x + 1,
                    inner.y,
                    text::truncate(node.label, inner.width.saturating_sub(1) as usize),
                    Style::new()
                        .fg(t.text(TextRole::Primary))
                        .bg(t.surface(Surface::Card)),
                );
            }
        });
    }
}
