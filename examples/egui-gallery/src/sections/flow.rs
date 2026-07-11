//! Flow: the auto-layout flowchart (always available — no feature gate).

use forge_egui::prelude::*;
use forge_egui::widgets::Tone;

pub fn draw(ui: &mut egui::Ui) {
    Card::new().title("Build pipeline").show(ui, |ui| {
        let nodes = [
            FlowNode::new("checkout", "Checkout"),
            FlowNode::new("lint", "Lint"),
            FlowNode::new("build", "Build").tone(Tone::Accent),
            FlowNode::new("test", "Test").tone(Tone::Warning),
            FlowNode::new("package", "Package"),
            FlowNode::new("deploy", "Deploy").tone(Tone::Success),
        ];
        let edges = [
            FlowEdge::new("checkout", "lint"),
            FlowEdge::new("checkout", "build"),
            FlowEdge::new("lint", "test"),
            FlowEdge::new("build", "test"),
            FlowEdge::new("test", "package").label("on green"),
            FlowEdge::new("package", "deploy").broken(true),
        ];
        egui::ScrollArea::horizontal()
            .id_salt("flow-scroll")
            .show(ui, |ui| {
                let _ = Flowchart::new(&nodes, &edges).show(ui);
            });
        ui.add_space(8.0);
        let t = Theme::of(ui.ctx());
        ui.label(
            egui::RichText::new("Layered auto-layout · elbow edges · dashed danger = broken link")
                .font(t.font(
                    ui.ctx(),
                    forge_egui::theme::FontWeight::Regular,
                    t.type_scale.xs,
                ))
                .color(t.fg[2]),
        );
    });
}
