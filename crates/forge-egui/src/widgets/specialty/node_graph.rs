//! Interactive node-graph editor — the egui sibling of `@forge/graph`'s
//! `NodeGraph`. Typed ports, elbow edges, pan/zoom canvas, connect-by-drag.
//!
//! Like [`Kanban`](crate::widgets::Kanban), the widget cannot mutate your
//! graph: `show` returns the [`GraphEvent`]s the user requested this frame
//! (moves, connections, deletions, …) and the caller applies them to its own
//! `Vec<GraphNode>` / `Vec<GraphEdge>`.
//!
//! Interactions:
//! - drag empty canvas (or middle-drag anywhere) → pan; scroll → zoom at the
//!   pointer
//! - drag a node's title bar → [`GraphEvent::Moved`] per frame
//! - drag from an output port dot, drop on a type-compatible input →
//!   [`GraphEvent::Connected`] (incompatible targets tint danger, no event)
//! - click an edge near its midpoint to select it; click the × again (or
//!   press Delete) → [`GraphEvent::Disconnected`]
//! - click a node → [`GraphEvent::Selected`]; click the canvas → deselect;
//!   Delete with a node selected → [`GraphEvent::Deleted`]
//!
//! Delete/Backspace is only consumed while the pointer is over the canvas
//! and no text widget has focus.

use crate::theme::{FontWeight, Theme};
use crate::widgets::Tone;
use egui::{
    Color32, CornerRadius, Key, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType,
};

/* ---------------- Data model (owned by the app) --------------------------- */

/// Port value types, mirroring the web kit's `PortType`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PortType {
    /// Control flow — connects only to other triggers.
    Trigger,
    String,
    Number,
    Boolean,
    Object,
    Array,
    /// Wildcard data port — connects to any non-trigger type.
    #[default]
    Any,
}

impl PortType {
    /// Whether an output of this type may connect to an input of `to`.
    ///
    /// Rules (same as the web kit, with the trigger rule made explicit):
    /// same type always connects; `Any` connects to any *data* type on
    /// either side; `Trigger` connects only to `Trigger`.
    pub fn can_connect(self, to: PortType) -> bool {
        use PortType::*;
        match (self, to) {
            (Trigger, Trigger) => true,
            (Trigger, _) | (_, Trigger) => false,
            (a, b) => a == b || a == Any || b == Any,
        }
    }

    /// The port-dot colour. Mirrors the web `PORT_COLORS` table — every web
    /// entry maps 1:1 onto a Forge theme token (`trigger: --fg-0`,
    /// `string: --success`, `number: --info`, `boolean: --danger`,
    /// `object: --accent`, `array: --warning`, `any: --fg-3`), so nothing
    /// needs hardcoding.
    pub fn color(self, t: &Theme) -> Color32 {
        match self {
            PortType::Trigger => t.fg[0],
            PortType::String => t.success.base,
            PortType::Number => t.info.base,
            PortType::Boolean => t.danger.base,
            PortType::Object => t.accent.base,
            PortType::Array => t.warning.base,
            PortType::Any => t.fg[3],
        }
    }
}

/// A single input or output port on a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub name: String,
    pub ty: PortType,
}

impl Port {
    pub fn new(name: impl Into<String>, ty: PortType) -> Port {
        Port {
            name: name.into(),
            ty,
        }
    }
}

/// One node. `pos` is the top-left corner in canvas coordinates; `tone`
/// tints the title bar's leading edge.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub pos: Pos2,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub tone: Option<Tone>,
}

impl GraphNode {
    pub fn new(id: impl Into<String>, title: impl Into<String>, pos: Pos2) -> GraphNode {
        GraphNode {
            id: id.into(),
            title: title.into(),
            pos,
            inputs: Vec::new(),
            outputs: Vec::new(),
            tone: None,
        }
    }

    pub fn input(mut self, name: impl Into<String>, ty: PortType) -> Self {
        self.inputs.push(Port::new(name, ty));
        self
    }

    pub fn output(mut self, name: impl Into<String>, ty: PortType) -> Self {
        self.outputs.push(Port::new(name, ty));
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Node footprint in canvas units.
    pub(crate) fn size(&self) -> Vec2 {
        let rows = (self.inputs.len() + self.outputs.len()) as f32;
        let pad = if rows > 0.0 { 6.0 } else { 0.0 };
        Vec2::new(NODE_W, HEAD_H + rows * ROW_H + pad)
    }

    /// Canvas position of input dot `i` (left edge; input rows come first).
    pub(crate) fn in_anchor(&self, i: usize) -> Pos2 {
        Pos2::new(
            self.pos.x,
            self.pos.y + HEAD_H + ROW_H * i as f32 + ROW_H / 2.0,
        )
    }

    /// Canvas position of output dot `i` (right edge, after the input rows).
    pub(crate) fn out_anchor(&self, i: usize) -> Pos2 {
        Pos2::new(
            self.pos.x + NODE_W,
            self.pos.y + HEAD_H + ROW_H * (self.inputs.len() + i) as f32 + ROW_H / 2.0,
        )
    }
}

/// A connection: `(node id, output index)` → `(node id, input index)`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub from: (String, usize),
    pub to: (String, usize),
}

impl GraphEdge {
    pub fn new(
        from_node: impl Into<String>,
        from_port: usize,
        to_node: impl Into<String>,
        to_port: usize,
    ) -> GraphEdge {
        GraphEdge {
            from: (from_node.into(), from_port),
            to: (to_node.into(), to_port),
        }
    }
}

/// An in-flight connect-by-drag from an output port.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingConnection {
    /// `(node id, output index)` the drag started from.
    pub from: (String, usize),
    /// Type of the source output — decides target compatibility.
    pub ty: PortType,
}

/// Editor interaction state, owned by the app between frames.
#[derive(Clone, Debug)]
pub struct NodeGraphState {
    /// Selected node id, if any.
    pub selected: Option<String>,
    /// Selected edge (index into the `edges` slice), if any.
    pub selected_edge: Option<usize>,
    /// Canvas pan in screen pixels.
    pub pan: Vec2,
    /// Canvas zoom (1.0 = 1:1), clamped to [`ZOOM_MIN`]..=[`ZOOM_MAX`].
    pub zoom: f32,
    /// Connect-by-drag in progress, if any.
    pub pending: Option<PendingConnection>,
    /// Node-drag bookkeeping: `(node id, grab offset in canvas units)`.
    drag: Option<(String, Vec2)>,
}

impl Default for NodeGraphState {
    fn default() -> Self {
        NodeGraphState {
            selected: None,
            selected_edge: None,
            pan: Vec2::ZERO,
            zoom: 1.0,
            pending: None,
            drag: None,
        }
    }
}

/// What the user asked for this frame. Apply these to your own data.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEvent {
    /// A node was dragged (fires per frame during the drag).
    Moved { node: String, pos: Pos2 },
    /// A compatible output→input drop; append the edge.
    Connected { edge: GraphEdge },
    /// An edge was removed (index into the `edges` slice as passed in).
    Disconnected { index: usize },
    /// Node selection changed (`None` = canvas click deselected).
    Selected(Option<String>),
    /// Delete pressed with this node selected; remove it (and its edges).
    Deleted(String),
}

/* ---------------- Geometry ------------------------------------------------- */

pub(crate) const NODE_W: f32 = 180.0;
pub(crate) const HEAD_H: f32 = 30.0;
pub(crate) const ROW_H: f32 = 22.0;
const PORT_R: f32 = 4.0;
/// Screen-space hit radius for port dots and edge midpoints.
const HIT_R: f32 = 9.0;
const STUB: f32 = 16.0;
const GRID: f32 = 24.0;
pub(crate) const ZOOM_MIN: f32 = 0.25;
pub(crate) const ZOOM_MAX: f32 = 2.5;

/// Canvas → screen: `screen = origin + pan + canvas × zoom`.
pub(crate) fn to_screen(origin: Pos2, pan: Vec2, zoom: f32, p: Pos2) -> Pos2 {
    origin + pan + p.to_vec2() * zoom
}

/// Screen → canvas (inverse of [`to_screen`]).
pub(crate) fn to_canvas(origin: Pos2, pan: Vec2, zoom: f32, s: Pos2) -> Pos2 {
    (((s - origin) - pan) / zoom).to_pos2()
}

/// Zoom by `factor` keeping the canvas point under `pointer` (screen pixels
/// relative to the canvas origin) fixed. Returns the new `(pan, zoom)`.
pub(crate) fn zoom_at(pan: Vec2, zoom: f32, pointer: Vec2, factor: f32) -> (Vec2, f32) {
    let new_zoom = (zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
    let new_pan = pointer - (pointer - pan) * (new_zoom / zoom);
    (new_pan, new_zoom)
}

/// Orthogonal elbow polyline from an output anchor (exits rightward) to an
/// input anchor (enters leftward) — same routing as the web kit's
/// `elbowPoints` and visually consistent with [`Flowchart`](super::Flowchart).
pub(crate) fn elbow_points(a: Pos2, b: Pos2) -> Vec<Pos2> {
    if b.x - a.x >= 2.0 * STUB {
        if (a.y - b.y).abs() < 0.5 {
            return vec![a, b];
        }
        let mx = (a.x + b.x) / 2.0;
        return vec![a, Pos2::new(mx, a.y), Pos2::new(mx, b.y), b];
    }
    // Backward: detour around via stubs.
    let my = (a.y + b.y) / 2.0;
    vec![
        a,
        Pos2::new(a.x + STUB, a.y),
        Pos2::new(a.x + STUB, my),
        Pos2::new(b.x - STUB, my),
        Pos2::new(b.x - STUB, b.y),
        b,
    ]
}

/// Point at half the total arc length of a polyline — where the edge's
/// select/× affordance lives.
pub(crate) fn polyline_midpoint(pts: &[Pos2]) -> Pos2 {
    let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    if total <= f32::EPSILON {
        return pts.first().copied().unwrap_or(Pos2::ZERO);
    }
    let mut remaining = total / 2.0;
    for w in pts.windows(2) {
        let d = w[0].distance(w[1]);
        if remaining <= d {
            return w[0] + (w[1] - w[0]) * (remaining / d.max(f32::EPSILON));
        }
        remaining -= d;
    }
    *pts.last().unwrap()
}

/* ---------------- Widget ---------------------------------------------------- */

/// The node-graph editor: `NodeGraph::new(&mut state, &nodes, &edges).show(ui)`.
pub struct NodeGraph<'a> {
    state: &'a mut NodeGraphState,
    nodes: &'a [GraphNode],
    edges: &'a [GraphEdge],
    height: f32,
}

impl<'a> NodeGraph<'a> {
    pub fn new(
        state: &'a mut NodeGraphState,
        nodes: &'a [GraphNode],
        edges: &'a [GraphEdge],
    ) -> NodeGraph<'a> {
        NodeGraph {
            state,
            nodes,
            edges,
            height: 420.0,
        }
    }

    /// Canvas height in points (default 420).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Vec<GraphEvent> {
        let t = Theme::of(ui.ctx());
        let Self {
            state,
            nodes,
            edges,
            height,
        } = self;
        let mut events: Vec<GraphEvent> = Vec::new();

        let size = Vec2::new(ui.available_width().max(120.0), height);
        let (rect, canvas) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if !ui.is_rect_visible(rect) {
            return events;
        }
        let painter = ui.painter_at(rect);
        let hovered_canvas = ui.rect_contains_pointer(rect);

        // Drop stale selections up front (the caller may have edited the graph).
        if let Some(i) = state.selected_edge {
            if i >= edges.len() {
                state.selected_edge = None;
            }
        }
        if let Some(id) = &state.selected {
            if !nodes.iter().any(|n| &n.id == id) {
                state.selected = None;
            }
        }

        // --- Pan (empty-canvas drag or middle-drag anywhere) + zoom (scroll).
        if hovered_canvas && ui.input(|i| i.pointer.middle_down()) {
            state.pan += ui.input(|i| i.pointer.delta());
        } else if canvas.dragged() {
            state.pan += canvas.drag_delta();
        }
        if hovered_canvas {
            // Consume the scroll so an enclosing ScrollArea doesn't also move.
            let (scroll, pinch) = ui.input_mut(|i| {
                let d = i.smooth_scroll_delta.y;
                let z = i.zoom_delta();
                if d != 0.0 {
                    i.smooth_scroll_delta = Vec2::ZERO;
                }
                (d, z)
            });
            let factor = (scroll * 0.003).exp() * pinch;
            if factor != 1.0 {
                if let Some(p) = ui.ctx().pointer_latest_pos() {
                    let (pan, zoom) = zoom_at(state.pan, state.zoom, p - rect.min, factor);
                    state.pan = pan;
                    state.zoom = zoom;
                }
            }
        }
        let (origin, pan, zoom) = (rect.min, state.pan, state.zoom);
        let ts = |p: Pos2| to_screen(origin, pan, zoom, p);

        // --- Canvas surface + dotted grid (pans/zooms with the content).
        let canvas_radius = CornerRadius::same(t.radius.lg as u8);
        painter.rect_filled(rect, canvas_radius, t.bg[0]);
        draw_grid(&painter, rect, pan, zoom, t.fg[3].gamma_multiply(0.45));

        // --- Edges (under nodes). Remember screen midpoints for hit-tests.
        let node_by_id = |id: &str| nodes.iter().find(|n| n.id == id);
        let mut edge_mids: Vec<Option<Pos2>> = Vec::with_capacity(edges.len());
        for (idx, edge) in edges.iter().enumerate() {
            let (Some(from), Some(to)) = (node_by_id(&edge.from.0), node_by_id(&edge.to.0)) else {
                edge_mids.push(None);
                continue;
            };
            let (Some(out_port), Some(_)) =
                (from.outputs.get(edge.from.1), to.inputs.get(edge.to.1))
            else {
                edge_mids.push(None);
                continue;
            };
            let pts: Vec<Pos2> =
                elbow_points(from.out_anchor(edge.from.1), to.in_anchor(edge.to.1))
                    .into_iter()
                    .map(ts)
                    .collect();
            let selected = state.selected_edge == Some(idx);
            let stroke = if selected {
                Stroke::new(2.5, t.accent.base)
            } else {
                Stroke::new(1.5, out_port.ty.color(&t))
            };
            edge_mids.push(Some(polyline_midpoint(&pts)));
            painter.add(Shape::line(pts, stroke));
        }

        // --- Nodes: paint + interact (registered after the canvas → on top).
        for node in nodes {
            let base_id = ui.id().with(("ng-node", &node.id));
            let nrect = Rect::from_min_size(ts(node.pos), node.size() * zoom);
            let head = Rect::from_min_size(nrect.min, Vec2::new(nrect.width(), HEAD_H * zoom));
            let selected = state.selected.as_deref() == Some(node.id.as_str());

            // Body.
            let radius = CornerRadius::same(t.radius.md as u8);
            painter.rect_filled(nrect, radius, t.bg[2]);
            let border = if selected {
                Stroke::new(1.5, t.accent.base)
            } else {
                Stroke::new(1.0, t.border.default)
            };
            painter.rect_stroke(nrect, radius, border, StrokeKind::Inside);

            // Tone bar + title.
            let mut title_x = nrect.min.x + 10.0 * zoom;
            if let Some(tone) = node.tone {
                let (base, _, _) = tone.triple(&t);
                let bar = Rect::from_min_max(
                    nrect.min + Vec2::new(1.5 * zoom, 4.0 * zoom),
                    Pos2::new(nrect.min.x + 4.5 * zoom, head.max.y - 4.0 * zoom),
                );
                painter.rect_filled(bar, CornerRadius::same(1), base);
                title_x += 2.0 * zoom;
            }
            let title_font = t.font(
                ui.ctx(),
                FontWeight::Medium,
                (t.type_scale.sm * zoom).max(5.0),
            );
            let title = painter.layout_no_wrap(node.title.clone(), title_font, t.fg[0]);
            painter.galley(
                Pos2::new(title_x, head.center().y - title.size().y / 2.0),
                title,
                t.fg[0],
            );
            if !node.inputs.is_empty() || !node.outputs.is_empty() {
                painter.line_segment(
                    [
                        Pos2::new(nrect.min.x + 1.0, head.max.y),
                        Pos2::new(nrect.max.x - 1.0, head.max.y),
                    ],
                    Stroke::new(1.0, t.border.subtle),
                );
            }

            // Port rows.
            let label_font = t.font(
                ui.ctx(),
                FontWeight::Regular,
                (t.type_scale.xs * zoom).max(5.0),
            );
            let dot_r = (PORT_R * zoom).max(2.0);
            for (i, port) in node.inputs.iter().enumerate() {
                let a = ts(node.in_anchor(i));
                painter.circle(a, dot_r, port.ty.color(&t), Stroke::new(1.0, t.bg[0]));
                let g = painter.layout_no_wrap(port.name.clone(), label_font.clone(), t.fg[1]);
                painter.galley(
                    Pos2::new(nrect.min.x + 10.0 * zoom, a.y - g.size().y / 2.0),
                    g,
                    t.fg[1],
                );
            }
            for (i, port) in node.outputs.iter().enumerate() {
                let a = ts(node.out_anchor(i));
                painter.circle(a, dot_r, port.ty.color(&t), Stroke::new(1.0, t.bg[0]));
                let g = painter.layout_no_wrap(port.name.clone(), label_font.clone(), t.fg[1]);
                painter.galley(
                    Pos2::new(
                        nrect.max.x - 10.0 * zoom - g.size().x,
                        a.y - g.size().y / 2.0,
                    ),
                    g,
                    t.fg[1],
                );
            }

            // Interaction: body click selects; title bar drags (primary only,
            // so middle-drag pans even when it starts on a node).
            let body_resp = ui.interact(nrect, base_id.with("body"), Sense::click());
            body_resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &node.title));
            let head_resp = ui
                .interact(head, base_id.with("head"), Sense::click_and_drag())
                .on_hover_cursor(egui::CursorIcon::Grab);

            let mut select = body_resp.clicked() || head_resp.clicked();
            if head_resp.drag_started_by(egui::PointerButton::Primary) {
                if let Some(p) = head_resp.interact_pointer_pos() {
                    let grab = to_canvas(origin, pan, zoom, p) - node.pos;
                    state.drag = Some((node.id.clone(), grab));
                }
                select = true;
            }
            if head_resp.dragged_by(egui::PointerButton::Primary) {
                if let (Some((id, grab)), Some(p)) = (&state.drag, head_resp.interact_pointer_pos())
                {
                    if id == &node.id {
                        events.push(GraphEvent::Moved {
                            node: node.id.clone(),
                            pos: to_canvas(origin, pan, zoom, p) - *grab,
                        });
                    }
                }
            }
            if head_resp.drag_stopped() {
                state.drag = None;
            }
            if select && !selected {
                state.selected = Some(node.id.clone());
                state.selected_edge = None;
                events.push(GraphEvent::Selected(Some(node.id.clone())));
            }

            // Output dots start a pending connection.
            for (i, port) in node.outputs.iter().enumerate() {
                let a = ts(node.out_anchor(i));
                let hit = Rect::from_center_size(a, Vec2::splat(HIT_R * 2.0));
                let pr = ui.interact(hit, base_id.with(("out", i)), Sense::drag());
                if pr.hovered() || pr.dragged() {
                    painter.circle_stroke(a, dot_r + 2.5, Stroke::new(1.5, port.ty.color(&t)));
                }
                if pr.drag_started_by(egui::PointerButton::Primary) {
                    state.pending = Some(PendingConnection {
                        from: (node.id.clone(), i),
                        ty: port.ty,
                    });
                }
            }
        }

        // --- Pending connection: elbow preview + drop resolution.
        if let Some(pending) = state.pending.clone() {
            let start = node_by_id(&pending.from.0).map(|n| n.out_anchor(pending.from.1));
            let pointer = ui.ctx().pointer_latest_pos();
            if let (Some(start_c), Some(p)) = (start, pointer) {
                // Nearest input dot under the pointer, compatible or not.
                let mut target: Option<(&GraphNode, usize, bool)> = None;
                for n in nodes {
                    for (i, port) in n.inputs.iter().enumerate() {
                        if ts(n.in_anchor(i)).distance(p) <= HIT_R {
                            let ok = n.id != pending.from.0 && pending.ty.can_connect(port.ty);
                            target = Some((n, i, ok));
                        }
                    }
                }
                let end_c = match target {
                    Some((n, i, true)) => n.in_anchor(i),
                    _ => to_canvas(origin, pan, zoom, p),
                };
                let pts: Vec<Pos2> = elbow_points(start_c, end_c).into_iter().map(ts).collect();
                let color = match target {
                    Some((_, _, false)) => t.danger.base,
                    _ => pending.ty.color(&t),
                };
                painter.add(Shape::dashed_line(&pts, Stroke::new(1.5, color), 6.0, 4.0));
                if let Some((n, i, ok)) = target {
                    let ring = if ok { t.accent.base } else { t.danger.base };
                    painter.circle_stroke(
                        ts(n.in_anchor(i)),
                        (PORT_R * zoom).max(2.0) + 3.0,
                        Stroke::new(1.5, ring),
                    );
                }
                if ui.input(|i| i.pointer.any_released()) {
                    if let Some((n, i, true)) = target {
                        events.push(GraphEvent::Connected {
                            edge: GraphEdge {
                                from: pending.from.clone(),
                                to: (n.id.clone(), i),
                            },
                        });
                    }
                    state.pending = None;
                }
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    state.pending = None;
                }
                ui.ctx().request_repaint();
            } else {
                state.pending = None;
            }
        }

        // --- Edge midpoint affordance (topmost interact): click selects,
        // clicking the shown × disconnects.
        for (idx, mid) in edge_mids.iter().enumerate() {
            let Some(mid) = *mid else { continue };
            if !rect.contains(mid) {
                continue;
            }
            let er = ui.interact(
                Rect::from_center_size(mid, Vec2::splat(HIT_R * 2.0)),
                ui.id().with(("ng-edge", idx)),
                Sense::click(),
            );
            let selected = state.selected_edge == Some(idx);
            if er.clicked() {
                if selected {
                    events.push(GraphEvent::Disconnected { index: idx });
                    state.selected_edge = None;
                } else {
                    state.selected_edge = Some(idx);
                    if state.selected.take().is_some() {
                        events.push(GraphEvent::Selected(None));
                    }
                }
            }
            if er.hovered() || selected {
                let tone = if selected { t.danger.base } else { t.fg[1] };
                painter.circle(mid, 7.0, t.bg[3], Stroke::new(1.0, t.border.strong));
                let d = 2.8;
                let s = Stroke::new(1.4, tone);
                painter.line_segment([mid + Vec2::new(-d, -d), mid + Vec2::new(d, d)], s);
                painter.line_segment([mid + Vec2::new(-d, d), mid + Vec2::new(d, -d)], s);
            }
        }

        // --- Canvas click deselects.
        if canvas.clicked() {
            if state.selected.take().is_some() {
                events.push(GraphEvent::Selected(None));
            }
            state.selected_edge = None;
        }

        // --- Delete key (pointer over the canvas, no text widget focused).
        let delete = ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
        if delete && hovered_canvas && ui.ctx().memory(|m| m.focused().is_none()) {
            if let Some(idx) = state.selected_edge.take() {
                events.push(GraphEvent::Disconnected { index: idx });
            } else if let Some(id) = state.selected.take() {
                events.push(GraphEvent::Deleted(id));
            }
        }

        // Canvas frame on top of the content.
        painter.rect_stroke(
            rect,
            canvas_radius,
            Stroke::new(1.0, t.border.subtle),
            StrokeKind::Inside,
        );

        events
    }
}

/// Dotted background grid: dots at canvas multiples of [`GRID`], so they
/// track pan and zoom. Spacing doubles while too dense on screen.
fn draw_grid(painter: &egui::Painter, rect: Rect, pan: Vec2, zoom: f32, color: Color32) {
    let mut sp = GRID * zoom;
    while sp < 14.0 {
        sp *= 2.0;
    }
    let mut y = rect.min.y + pan.y.rem_euclid(sp) - sp;
    while y <= rect.max.y + sp {
        let mut x = rect.min.x + pan.x.rem_euclid(sp) - sp;
        while x <= rect.max.x + sp {
            painter.circle_filled(Pos2::new(x, y), 1.0, color);
            x += sp;
        }
        y += sp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_compatibility_matrix() {
        use PortType::*;
        let data = [String, Number, Boolean, Object, Array];
        // Same type always connects.
        for ty in [Trigger, String, Number, Boolean, Object, Array, Any] {
            assert!(ty.can_connect(ty), "{ty:?} should connect to itself");
        }
        // Any ↔ data types, both directions.
        for ty in data {
            assert!(Any.can_connect(ty));
            assert!(ty.can_connect(Any));
        }
        // Distinct data types never connect.
        for a in data {
            for b in data {
                if a != b {
                    assert!(!a.can_connect(b), "{a:?} must not connect to {b:?}");
                }
            }
        }
        // Trigger only to trigger — not even Any.
        for ty in data {
            assert!(!Trigger.can_connect(ty));
            assert!(!ty.can_connect(Trigger));
        }
        assert!(!Trigger.can_connect(Any));
        assert!(!Any.can_connect(Trigger));
    }

    #[test]
    fn screen_canvas_round_trip() {
        let origin = Pos2::new(120.0, 80.0);
        let pan = Vec2::new(-37.5, 12.25);
        let zoom = 1.75;
        for p in [Pos2::ZERO, Pos2::new(240.0, 160.0), Pos2::new(-55.5, 999.0)] {
            let s = to_screen(origin, pan, zoom, p);
            let back = to_canvas(origin, pan, zoom, s);
            assert!((back - p).length() < 1e-3, "{p:?} → {s:?} → {back:?}");
        }
    }

    #[test]
    fn zoom_at_keeps_pointer_point_fixed() {
        let origin = Pos2::new(10.0, 10.0);
        let (pan, zoom) = (Vec2::new(40.0, -20.0), 1.0);
        let pointer = Vec2::new(300.0, 150.0);
        let under = to_canvas(origin, pan, zoom, origin + pointer);
        let (pan2, zoom2) = zoom_at(pan, zoom, pointer, 1.4);
        assert!((zoom2 - 1.4).abs() < 1e-6);
        let under2 = to_canvas(origin, pan2, zoom2, origin + pointer);
        assert!((under2 - under).length() < 1e-3);
    }

    #[test]
    fn zoom_at_clamps_to_range() {
        let (_, z_lo) = zoom_at(Vec2::ZERO, ZOOM_MIN, Vec2::ZERO, 0.01);
        let (_, z_hi) = zoom_at(Vec2::ZERO, ZOOM_MAX, Vec2::ZERO, 100.0);
        assert_eq!(z_lo, ZOOM_MIN);
        assert_eq!(z_hi, ZOOM_MAX);
    }

    #[test]
    fn elbow_routes_forward_and_backward() {
        // Forward with vertical offset: 4 points, mid-x elbow.
        let pts = elbow_points(Pos2::new(0.0, 0.0), Pos2::new(100.0, 50.0));
        assert_eq!(pts.len(), 4);
        assert_eq!(pts[1].x, 50.0);
        assert_eq!(pts[2].x, 50.0);
        // Straight horizontal: just the two endpoints.
        let pts = elbow_points(Pos2::new(0.0, 0.0), Pos2::new(100.0, 0.0));
        assert_eq!(pts.len(), 2);
        // Backward: 6-point detour through the stubs.
        let pts = elbow_points(Pos2::new(100.0, 0.0), Pos2::new(0.0, 50.0));
        assert_eq!(pts.len(), 6);
        assert_eq!(pts[1].x, 100.0 + STUB);
        assert_eq!(pts[4].x, -STUB);
    }

    #[test]
    fn midpoint_bisects_arc_length() {
        // Straight segment → center.
        let mid = polyline_midpoint(&[Pos2::new(0.0, 0.0), Pos2::new(10.0, 0.0)]);
        assert!((mid - Pos2::new(5.0, 0.0)).length() < 1e-4);
        // L-shape 10 + 10: half-way (10) lands exactly on the corner.
        let mid = polyline_midpoint(&[
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(10.0, 10.0),
        ]);
        assert!((mid - Pos2::new(10.0, 0.0)).length() < 1e-4);
        // Elbow edge: midpoint sits on the vertical middle segment.
        let pts = elbow_points(Pos2::new(0.0, 0.0), Pos2::new(100.0, 60.0));
        let mid = polyline_midpoint(&pts);
        assert!((mid.x - 50.0).abs() < 1e-4);
        assert!(mid.y > 0.0 && mid.y < 60.0);
    }

    #[test]
    fn anchors_stack_inputs_then_outputs() {
        let n = GraphNode::new("n", "N", Pos2::new(10.0, 20.0))
            .input("a", PortType::Any)
            .input("b", PortType::Number)
            .output("out", PortType::Object);
        assert_eq!(n.in_anchor(0), Pos2::new(10.0, 20.0 + HEAD_H + ROW_H / 2.0));
        assert_eq!(
            n.in_anchor(1),
            Pos2::new(10.0, 20.0 + HEAD_H + ROW_H + ROW_H / 2.0)
        );
        // Output rows come after the two input rows, on the right edge.
        assert_eq!(
            n.out_anchor(0),
            Pos2::new(10.0 + NODE_W, 20.0 + HEAD_H + 2.0 * ROW_H + ROW_H / 2.0)
        );
        assert_eq!(n.size().y, HEAD_H + 3.0 * ROW_H + 6.0);
    }
}
