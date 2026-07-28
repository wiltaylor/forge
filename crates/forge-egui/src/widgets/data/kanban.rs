//! Drag-and-drop kanban board. Like forge-tui's, the widget cannot mutate
//! your board: dropping a card *requests* a [`KanbanMove`] from `show` and
//! you apply it to your own data.
//!
//! `KanbanMove::index` is the insertion position in the target column *as
//! currently displayed*. When moving within one column, remove the card
//! first and decrement `index` if it was above the removal point (see the
//! gallery's board section).

use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use crate::widgets::Tone;
use egui::{CornerRadius, DragAndDrop, Frame, Margin, Sense, Stroke, StrokeKind, Ui, Vec2};

/// One card: stable id, title, optional tone badge.
#[derive(Clone, Debug)]
pub struct KanbanCard {
    pub id: String,
    pub title: String,
    pub badge: Option<(String, Tone)>,
}

impl KanbanCard {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> KanbanCard {
        KanbanCard {
            id: id.into(),
            title: title.into(),
            badge: None,
        }
    }

    pub fn badge(mut self, label: impl Into<String>, tone: Tone) -> Self {
        self.badge = Some((label.into(), tone));
        self
    }
}

/// One column: title + owned cards.
#[derive(Clone, Debug, Default)]
pub struct KanbanColumn {
    pub title: String,
    pub cards: Vec<KanbanCard>,
}

impl KanbanColumn {
    pub fn new(title: impl Into<String>) -> KanbanColumn {
        KanbanColumn {
            title: title.into(),
            cards: Vec::new(),
        }
    }

    pub fn card(mut self, card: KanbanCard) -> Self {
        self.cards.push(card);
        self
    }

    pub fn cards(mut self, cards: Vec<KanbanCard>) -> Self {
        self.cards = cards;
        self
    }
}

/// The move requested by a drop. Apply it to your own columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KanbanMove {
    /// Id of the dragged card.
    pub card: String,
    /// Source column index.
    pub from: usize,
    /// Target column index.
    pub to: usize,
    /// Insertion index in the target column (pre-removal display order).
    pub index: usize,
}

/// Board interaction state: the id of the card being dragged, if any.
#[derive(Clone, Debug, Default)]
pub struct KanbanState {
    pub drag: Option<String>,
}

/// The dnd payload riding egui's [`DragAndDrop`] plugin.
#[derive(Clone, Debug)]
struct DragPayload {
    card: String,
    from: usize,
}

/// Insertion index from the pointer's y position given the top/bottom of
/// each card in the column: before the first midpoint it crosses.
pub(crate) fn insert_index(card_ranges: &[(f32, f32)], pointer_y: f32) -> usize {
    for (i, (top, bottom)) in card_ranges.iter().enumerate() {
        if pointer_y < (top + bottom) / 2.0 {
            return i;
        }
    }
    card_ranges.len()
}

/// The board widget.
pub struct Kanban<'a> {
    state: &'a mut KanbanState,
    columns: &'a [KanbanColumn],
    min_height: f32,
}

impl<'a> Kanban<'a> {
    pub fn new(state: &'a mut KanbanState, columns: &'a [KanbanColumn]) -> Kanban<'a> {
        Kanban {
            state,
            columns,
            min_height: 220.0,
        }
    }

    /// Minimum column-well height (default 220).
    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<KanbanMove> {
        let t = Theme::of(ui.ctx());
        let Self {
            state,
            columns,
            min_height,
        } = self;
        let mut requested: Option<KanbanMove> = None;
        state.drag = None;

        let gap = t.space.x(3.0);
        let n = columns.len().max(1) as f32;
        let col_w = ((ui.available_width() - gap * (n - 1.0)) / n).max(80.0);
        let payload = DragAndDrop::payload::<DragPayload>(ui.ctx());
        let released = ui.input(|i| i.pointer.any_released());
        let pointer = ui.ctx().pointer_interact_pos();

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (ci, column) in columns.iter().enumerate() {
                let well = Frame::new()
                    .fill(t.bg[1])
                    .stroke(Stroke::new(1.0, t.border.subtle))
                    .corner_radius(CornerRadius::same(t.radius.lg as u8))
                    .inner_margin(Margin::same(t.space.x(2.0) as i8));
                let inner = well.show(ui, |ui| {
                    // The well sits in a horizontal row — stack its content.
                    ui.vertical(|ui| {
                        ui.set_width(col_w - t.space.x(4.0));
                        ui.set_min_height(min_height);
                        // Header: title + count badge.
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&column.title)
                                    .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm))
                                    .color(t.fg[1]),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    count_badge(ui, &t, column.cards.len());
                                },
                            );
                        });
                        ui.add_space(t.space.x(2.0));

                        // Cards (each a drag source).
                        let mut ranges: Vec<(f32, f32)> = Vec::with_capacity(column.cards.len());
                        for card in &column.cards {
                            let id = ui.id().with(("kanban-card", &card.id));
                            if ui.ctx().is_being_dragged(id) {
                                state.drag = Some(card.id.clone());
                            }
                            let r = ui.dnd_drag_source(
                                id,
                                DragPayload {
                                    card: card.id.clone(),
                                    from: ci,
                                },
                                |ui| card_ui(ui, &t, card),
                            );
                            let rect = r.response.rect;
                            ranges.push((rect.min.y, rect.max.y));
                            ui.add_space(t.space.x(1.5));
                        }
                        ranges
                    })
                    .inner
                });

                // Drop handling on the whole well.
                let col_rect = inner.response.rect;
                if let (Some(payload), Some(pos)) = (payload.as_deref(), pointer) {
                    if col_rect.contains(pos) {
                        let index = insert_index(&inner.inner, pos.y);
                        // Indicator line at the insertion gap.
                        let y = match (inner.inner.get(index), index.checked_sub(1)) {
                            (Some((top, _)), _) => top - 3.0,
                            (None, Some(prev)) => inner.inner[prev].1 + 3.0,
                            (None, None) => col_rect.min.y + 34.0,
                        };
                        let x0 = col_rect.min.x + 6.0;
                        let x1 = col_rect.max.x - 6.0;
                        ui.painter().line_segment(
                            [egui::pos2(x0, y), egui::pos2(x1, y)],
                            Stroke::new(2.0, t.accent.base),
                        );
                        if released {
                            requested = Some(KanbanMove {
                                card: payload.card.clone(),
                                from: payload.from,
                                to: ci,
                                index,
                            });
                        }
                    }
                }
            }
        });

        if requested.is_some() {
            DragAndDrop::clear_payload(ui.ctx());
        }
        requested
    }
}

fn card_ui(ui: &mut Ui, t: &Theme, card: &KanbanCard) {
    let frame = Frame::new()
        .fill(t.bg[2])
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(t.space.x(2.0) as i8));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new(&card.title)
                .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                .color(t.fg[0]),
        );
        if let Some((label, tone)) = &card.badge {
            ui.add_space(t.space.x(1.0));
            let _ = crate::widgets::primitives::Badge::new(label)
                .tone(*tone)
                .show(ui);
        }
    });
}

fn count_badge(ui: &mut Ui, t: &Theme, count: usize) {
    let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs);
    let text = count.to_string();
    let g = util::galley(ui, &text, font, t.fg[2]);
    let size = Vec2::new((g.size().x + 12.0).max(18.0), 16.0);
    let (rect, _r) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), t.bg[3]);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(8),
            Stroke::new(1.0, t.border.subtle),
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(rect.center() - g.size() / 2.0, g, t.fg[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_index_lands_at_midpoint_gaps() {
        // Three cards at y 0..20, 30..50, 60..80.
        let ranges = [(0.0, 20.0), (30.0, 50.0), (60.0, 80.0)];
        assert_eq!(insert_index(&ranges, -5.0), 0);
        assert_eq!(insert_index(&ranges, 5.0), 0); // above first midpoint
        assert_eq!(insert_index(&ranges, 15.0), 1); // past first midpoint
        assert_eq!(insert_index(&ranges, 40.0), 2);
        assert_eq!(insert_index(&ranges, 75.0), 3);
        assert_eq!(insert_index(&ranges, 1000.0), 3);
    }

    #[test]
    fn insert_index_on_empty_column_is_zero() {
        assert_eq!(insert_index(&[], 42.0), 0);
    }
}
