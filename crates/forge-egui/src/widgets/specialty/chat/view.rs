//! The transcript: a stick-to-bottom scroll of message bubbles, tool-call
//! boxes, dividers, and the typing indicator.

use super::{ChatItem, Role, ToolStatus};
use crate::theme::{FontWeight, Theme};
use crate::widgets::specialty::markdown::Markdown;
use egui::{Align, CornerRadius, Frame, Layout, Margin, Pos2, RichText, Sense, Stroke, Ui, Vec2};

/// Scroll state: `stick` follows the tail until the user scrolls up.
#[derive(Clone, Copy, Debug)]
pub struct ChatViewState {
    pub stick: bool,
}

impl Default for ChatViewState {
    fn default() -> ChatViewState {
        ChatViewState { stick: true }
    }
}

impl ChatViewState {
    pub fn new() -> ChatViewState {
        ChatViewState::default()
    }
}

/// Transcript view: `ChatView::new(&items).show(ui, &mut state)`.
#[derive(Clone, Debug)]
pub struct ChatView<'a> {
    items: &'a [ChatItem],
    max_height: Option<f32>,
}

impl<'a> ChatView<'a> {
    pub fn new(items: &'a [ChatItem]) -> ChatView<'a> {
        ChatView {
            items,
            max_height: None,
        }
    }

    /// Cap the transcript height (defaults to the available height).
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    pub fn show(self, ui: &mut Ui, state: &mut ChatViewState) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let height = self.max_height.unwrap_or_else(|| ui.available_height());
        let out = egui::ScrollArea::vertical()
            .id_salt("forge-chat-view")
            .max_height(height)
            .auto_shrink([false, true])
            .stick_to_bottom(state.stick)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = t.space.x(3.0);
                for (i, item) in self.items.iter().enumerate() {
                    ui.push_id(i, |ui| render_item(ui, &t, item));
                }
            });
        // Re-stick when the user is (back) at the tail; unpin when they
        // scroll up.
        let max_offset = (out.content_size.y - out.inner_rect.height()).max(0.0);
        state.stick = out.state.offset.y >= max_offset - 4.0;
        ui.interact(out.inner_rect, out.id.with("resp"), Sense::hover())
    }
}

fn render_item(ui: &mut Ui, t: &Theme, item: &ChatItem) {
    match item {
        ChatItem::Message {
            role,
            name,
            time,
            markdown,
        } => message(ui, t, *role, name.as_deref(), time.as_deref(), markdown),
        ChatItem::ToolCall {
            title,
            status,
            body,
        } => tool_call(ui, t, title, *status, body.as_deref()),
        ChatItem::Divider(label) => divider(ui, t, label),
        ChatItem::Typing => typing(ui, t),
    }
}

fn message(
    ui: &mut Ui,
    t: &Theme,
    role: Role,
    name: Option<&str>,
    time: Option<&str>,
    markdown: &str,
) {
    let (align, fill, stroke, default_name) = match role {
        Role::User => (Align::Max, t.accent.bg, Stroke::NONE, "You"),
        Role::Assistant => (
            Align::Min,
            t.bg[1],
            Stroke::new(1.0, t.border.subtle),
            "Assistant",
        ),
        Role::System => (
            Align::Min,
            t.bg[1],
            Stroke::new(1.0, t.border.subtle),
            "System",
        ),
    };
    ui.with_layout(Layout::top_down(align), |ui| {
        // Caption: name + time, xs.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                RichText::new(name.unwrap_or(default_name))
                    .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs))
                    .color(t.fg[2]),
            );
            if let Some(time) = time {
                ui.label(
                    RichText::new(time)
                        .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs))
                        .color(t.fg[3]),
                );
            }
        });
        let max_w = ui.available_width() * 0.7;
        Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(t.radius.lg as u8))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_max_width(max_w);
                let _ = Markdown::new(markdown).show(ui);
            });
    });
}

fn tool_call(ui: &mut Ui, t: &Theme, title: &str, status: ToolStatus, body: Option<&str>) {
    let open_id = ui.id().with("forge-tool-open");
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);

    Frame::new()
        .fill(t.bg[1])
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 20.0);
            let header = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let (glyph, color) = match status {
                    ToolStatus::Running => ("⟳", t.info.base),
                    ToolStatus::Ok => ("✓", t.success.base),
                    ToolStatus::Error => ("✗", t.danger.base),
                };
                ui.label(
                    RichText::new(glyph)
                        .font(t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.base))
                        .color(color),
                );
                ui.label(
                    RichText::new(title)
                        .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                        .color(t.fg[0]),
                );
                if body.is_some() {
                    ui.label(
                        RichText::new(if open { "▾" } else { "▸" })
                            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                            .color(t.fg[2]),
                    );
                }
                if status == ToolStatus::Running {
                    ui.ctx().request_repaint();
                }
            });
            if body.is_some() {
                let response =
                    ui.interact(header.response.rect, open_id.with("hdr"), Sense::click());
                if response.clicked() {
                    open = !open;
                }
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
            if open {
                if let Some(body) = body {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(body)
                            .font(t.mono(t.type_scale.sm))
                            .color(t.fg[2]),
                    );
                }
            }
        });

    ui.data_mut(|d| d.insert_temp(open_id, open));
}

fn divider(ui: &mut Ui, t: &Theme, label: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    // Eyebrow-style label: uppercase, tracked out with hair spaces.
    let spaced: String = label
        .to_uppercase()
        .chars()
        .flat_map(|c| [c, '\u{200A}'])
        .collect();
    let g = ui.painter().layout_no_wrap(
        spaced.trim_end_matches('\u{200A}').to_owned(),
        t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs),
        t.fg[2],
    );
    let cy = rect.center().y;
    let half_gap = g.size().x / 2.0 + 8.0;
    let stroke = Stroke::new(1.0, t.border.default);
    ui.painter()
        .hline(rect.min.x..=(rect.center().x - half_gap), cy, stroke);
    ui.painter()
        .hline((rect.center().x + half_gap)..=rect.max.x, cy, stroke);
    ui.painter().galley(
        Pos2::new(rect.center().x - g.size().x / 2.0, cy - g.size().y / 2.0),
        g,
        t.fg[2],
    );
}

fn typing(ui: &mut Ui, t: &Theme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 18.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let time = ui.input(|i| i.time);
    for k in 0..3 {
        let phase = (time * 3.0 - k as f64 * 0.45).sin() as f32 * 0.5 + 0.5;
        let alpha = 0.25 + 0.75 * phase;
        ui.painter().circle_filled(
            Pos2::new(rect.min.x + 6.0 + k as f32 * 12.0, rect.center().y),
            3.0,
            t.fg[2].gamma_multiply(alpha),
        );
    }
    ui.ctx().request_repaint();
}
