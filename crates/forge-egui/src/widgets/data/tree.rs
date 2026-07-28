//! Hierarchical expandable list with indent guides. Expansion and selection
//! are keyed by the node's stable string `id`, so tree edits don't scramble
//! the state (unlike index-path keying).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};
use std::collections::HashSet;

/// An owned tree node. Build once, keep in your app state.
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
    pub icon: Option<Glyph>,
}

impl TreeNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> TreeNode {
        TreeNode {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            icon: None,
        }
    }

    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }
}

/// Expansion set (by node id) + selected node id. Plain app-owned data.
#[derive(Clone, Debug, Default)]
pub struct TreeState {
    pub expanded: HashSet<String>,
    pub selected: Option<String>,
}

const INDENT: f32 = 16.0;
const DISC_W: f32 = 18.0;

/// The tree widget: `▸`/`▾` disclosures, indent guides, nav-style rows.
/// Row click selects (`Changed`); disclosure click toggles expansion.
pub struct Tree<'a> {
    state: &'a mut TreeState,
    roots: &'a [TreeNode],
}

impl<'a> Tree<'a> {
    pub fn new(state: &'a mut TreeState, roots: &'a [TreeNode]) -> Tree<'a> {
        Tree { state, roots }
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self { state, roots } = self;
        let mut outcome = Outcome::Ignored;
        let mut union: Option<Response> = None;
        ui.spacing_mut().item_spacing.y = 0.0;
        for node in roots {
            node_ui(ui, &t, state, node, 0, &mut outcome, &mut union);
        }
        let response = union.unwrap_or_else(|| {
            // Empty tree: allocate a zero-height placeholder response.
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 0.0), Sense::hover())
                .1
        });
        ForgeResponse::new(response, outcome)
    }
}

#[allow(clippy::too_many_arguments)]
fn node_ui(
    ui: &mut Ui,
    t: &Theme,
    state: &mut TreeState,
    node: &TreeNode,
    depth: usize,
    outcome: &mut Outcome,
    union: &mut Option<Response>,
) {
    let row_h = t.control.sm;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    let selected = state.selected.as_deref() == Some(node.id.as_str());
    let branch = !node.children.is_empty();
    resp.widget_info(|| {
        WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &node.label)
    });

    // Disclosure hit-area on top of the row.
    let disc_rect = Rect::from_min_size(
        egui::pos2(rect.min.x + depth as f32 * INDENT, rect.min.y),
        Vec2::new(DISC_W, row_h),
    );
    let disc = if branch {
        let d = ui.interact(disc_rect, resp.id.with("disc"), Sense::click());
        d.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Button, true, format!("toggle {}", node.id))
        });
        Some(d)
    } else {
        None
    };

    let disc_clicked = disc.as_ref().is_some_and(|d| d.clicked());
    if disc_clicked {
        if !state.expanded.remove(&node.id) {
            state.expanded.insert(node.id.clone());
        }
        *outcome = outcome.merge(Outcome::Changed);
    } else if resp.clicked() {
        if branch && selected {
            // Nav-row nicety: clicking an already-selected branch toggles it.
            if !state.expanded.remove(&node.id) {
                state.expanded.insert(node.id.clone());
            }
            *outcome = outcome.merge(Outcome::Changed);
        } else if !selected {
            state.selected = Some(node.id.clone());
            *outcome = outcome.merge(Outcome::Changed);
        }
    }
    let expanded = branch && state.expanded.contains(&node.id);

    if ui.is_rect_visible(rect) {
        let radius = egui::CornerRadius::same(t.radius.sm as u8);
        if selected {
            ui.painter().rect_filled(rect, radius, t.accent.bg);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, radius, t.bg[2]);
        }
        // Indent guides: one vertical hairline per ancestor level.
        for d in 0..depth {
            let gx = rect.min.x + d as f32 * INDENT + DISC_W / 2.0;
            ui.painter()
                .vline(gx, rect.y_range(), Stroke::new(1.0, t.border.subtle));
        }
        let mut x = rect.min.x + depth as f32 * INDENT;
        let cy = rect.center().y;
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base);
        if branch {
            let glyph = if expanded {
                Glyph::ChevronDown
            } else {
                Glyph::ChevronRight
            };
            let color = if disc.as_ref().is_some_and(|d| d.hovered()) {
                t.fg[0]
            } else {
                t.fg[2]
            };
            let g = util::galley(ui, glyph.as_str(), font.clone(), color);
            ui.painter().galley(
                egui::pos2(
                    disc_rect.center().x - g.size().x / 2.0,
                    cy - g.size().y / 2.0,
                ),
                g,
                color,
            );
        }
        x += DISC_W;
        if let Some(icon) = node.icon {
            let color = if selected { t.accent.fg } else { t.fg[2] };
            let g = util::galley(ui, icon.as_str(), font.clone(), color);
            let gw = g.size().x;
            ui.painter()
                .galley(egui::pos2(x, cy - g.size().y / 2.0), g, color);
            x += gw + 6.0;
        }
        let color = if selected {
            t.accent.fg
        } else if resp.hovered() {
            t.fg[0]
        } else {
            t.fg[1]
        };
        let g = util::galley(ui, &node.label, font, color);
        ui.painter()
            .galley(egui::pos2(x, cy - g.size().y / 2.0), g, color);
    }

    *union = Some(match union.take() {
        Some(u) => u.union(resp),
        None => resp,
    });

    if expanded {
        for child in &node.children {
            node_ui(ui, t, state, child, depth + 1, outcome, union);
        }
    }
}
