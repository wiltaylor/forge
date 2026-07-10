use forge_tui::prelude::*;
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, area: Rect, _ctx: &mut Ctx, t: &Theme) {
    if area.height < 4 {
        return;
    }
    frame.render_widget(
        Eyebrow::new("Flowchart — auto-layered, read-only").theme(t),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let nodes = [
        FlowNode::new("ingest", "ingest"),
        FlowNode::new("parse", "parse"),
        FlowNode::new("validate", "validate"),
        FlowNode::new("store", "doc store"),
        FlowNode::new("events", "event bus"),
        FlowNode::new("sse", "SSE"),
        FlowNode::new("ws", "WebSocket"),
    ];
    let edges = [
        FlowEdge::new("ingest", "parse"),
        FlowEdge::new("parse", "validate"),
        FlowEdge::new("validate", "store"),
        FlowEdge::new("validate", "events"),
        FlowEdge::new("events", "sse"),
        FlowEdge::new("events", "ws"),
    ];
    frame.render_widget(
        Flowchart::new(&nodes, &edges).theme(t),
        Rect::new(area.x, area.y + 2, area.width, area.height - 2),
    );
}
