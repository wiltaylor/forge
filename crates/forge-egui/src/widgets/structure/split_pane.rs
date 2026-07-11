//! Two panes with a draggable divider.

use crate::theme::Theme;
use egui::{CursorIcon, Sense, Stroke, Ui, Vec2};

/// Persistent divider position, owned by the app.
#[derive(Clone, Debug)]
pub struct SplitState {
    /// First pane's size in points.
    pub size: f32,
}

impl Default for SplitState {
    fn default() -> Self {
        SplitState { size: 280.0 }
    }
}

pub struct SplitPane<'a> {
    state: &'a mut SplitState,
    min: f32,
    vertical: bool,
    height: f32,
}

impl<'a> SplitPane<'a> {
    pub fn new(state: &'a mut SplitState) -> SplitPane<'a> {
        SplitPane {
            state,
            min: 160.0,
            vertical: false,
            height: 240.0,
        }
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    /// Stack panes vertically (divider is horizontal).
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Total height of the split region (horizontal splits).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(
        self,
        ui: &mut Ui,
        first: impl FnOnce(&mut Ui),
        second: impl FnOnce(&mut Ui),
    ) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let total = if self.vertical {
            self.height
        } else {
            ui.available_width()
        };
        let max = (total - self.min).max(self.min);
        self.state.size = self.state.size.clamp(self.min, max);

        let outer = Vec2::new(ui.available_width(), self.height);
        let (outer_rect, response) = ui.allocate_exact_size(outer, Sense::hover());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(outer_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );

        let (a_rect, divider_rect, b_rect) = if self.vertical {
            let y = outer_rect.min.y + self.state.size;
            (
                egui::Rect::from_min_max(outer_rect.min, egui::pos2(outer_rect.max.x, y - 3.0)),
                egui::Rect::from_min_max(
                    egui::pos2(outer_rect.min.x, y - 3.0),
                    egui::pos2(outer_rect.max.x, y + 3.0),
                ),
                egui::Rect::from_min_max(egui::pos2(outer_rect.min.x, y + 3.0), outer_rect.max),
            )
        } else {
            let x = outer_rect.min.x + self.state.size;
            (
                egui::Rect::from_min_max(outer_rect.min, egui::pos2(x - 3.0, outer_rect.max.y)),
                egui::Rect::from_min_max(
                    egui::pos2(x - 3.0, outer_rect.min.y),
                    egui::pos2(x + 3.0, outer_rect.max.y),
                ),
                egui::Rect::from_min_max(egui::pos2(x + 3.0, outer_rect.min.y), outer_rect.max),
            )
        };

        let divider = child.interact(
            divider_rect,
            child.id().with("forge-split-divider"),
            Sense::drag(),
        );
        if divider.dragged() {
            let delta = divider.drag_delta();
            self.state.size += if self.vertical { delta.y } else { delta.x };
            self.state.size = self.state.size.clamp(self.min, max);
        }
        let cursor = if self.vertical {
            CursorIcon::ResizeVertical
        } else {
            CursorIcon::ResizeHorizontal
        };
        let divider_hovered = divider.hovered() || divider.dragged();
        if divider_hovered {
            child.ctx().set_cursor_icon(cursor);
        }

        let mut a_ui = child.new_child(
            egui::UiBuilder::new()
                .max_rect(a_rect.shrink(4.0))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        a_ui.set_clip_rect(a_rect);
        first(&mut a_ui);
        let mut b_ui = child.new_child(
            egui::UiBuilder::new()
                .max_rect(b_rect.shrink(4.0))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        b_ui.set_clip_rect(b_rect);
        second(&mut b_ui);

        let line_color = if divider_hovered {
            t.accent.base
        } else {
            t.border.default
        };
        let center = divider_rect.center();
        if self.vertical {
            child.painter().line_segment(
                [
                    egui::pos2(divider_rect.min.x, center.y),
                    egui::pos2(divider_rect.max.x, center.y),
                ],
                Stroke::new(1.0, line_color),
            );
        } else {
            child.painter().line_segment(
                [
                    egui::pos2(center.x, divider_rect.min.y),
                    egui::pos2(center.x, divider_rect.max.y),
                ],
                Stroke::new(1.0, line_color),
            );
        }
        response
    }
}
