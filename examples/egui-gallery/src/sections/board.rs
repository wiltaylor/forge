//! Kanban board: drag cards between columns. The widget only *requests*
//! moves — this section applies each returned `KanbanMove` to its own data.

use forge_egui::prelude::*;
use forge_egui::widgets::{Kanban, KanbanCard, KanbanColumn, KanbanMove, KanbanState};

pub struct BoardState {
    pub columns: Vec<KanbanColumn>,
    pub kanban: KanbanState,
    last_move: Option<KanbanMove>,
}

impl Default for BoardState {
    fn default() -> Self {
        let columns = vec![
            KanbanColumn::new("Todo")
                .card(KanbanCard::new("t1", "Wire OIDC refresh flow").badge("auth", Tone::Info))
                .card(KanbanCard::new("t2", "Table column resizing"))
                .card(KanbanCard::new("t3", "Audit log retention").badge("blocked", Tone::Danger)),
            KanbanColumn::new("Doing")
                .card(KanbanCard::new("d1", "Kanban drag & drop").badge("m6", Tone::Accent))
                .card(KanbanCard::new("d2", "Deploy pipeline flakes").badge("ci", Tone::Warning)),
            KanbanColumn::new("Done")
                .card(KanbanCard::new("f1", "Theme palette lock").badge("shipped", Tone::Success))
                .card(KanbanCard::new("f2", "Focus ring pass")),
        ];
        BoardState {
            columns,
            kanban: KanbanState::default(),
            last_move: None,
        }
    }
}

/// Apply a requested move to the caller-owned columns. `mv.index` is the
/// insertion slot in display order, so a same-column move past the card's
/// old position shifts down by one after removal.
fn apply_move(columns: &mut [KanbanColumn], mv: &KanbanMove) {
    let Some(old) = columns[mv.from].cards.iter().position(|c| c.id == mv.card) else {
        return;
    };
    let card = columns[mv.from].cards.remove(old);
    let mut index = mv.index;
    if mv.from == mv.to && index > old {
        index -= 1;
    }
    let len = columns[mv.to].cards.len();
    columns[mv.to].cards.insert(index.min(len), card);
}

pub fn draw(ui: &mut egui::Ui, state: &mut BoardState) {
    let t = Theme::of(ui.ctx());

    Card::new()
        .title("Kanban — drag cards between columns")
        .show(ui, |ui| {
            if let Some(mv) = Kanban::new(&mut state.kanban, &state.columns)
                .min_height(300.0)
                .show(ui)
            {
                apply_move(&mut state.columns, &mv);
                state.last_move = Some(mv);
            }
            ui.add_space(8.0);
            let readout = match (&state.kanban.drag, &state.last_move) {
                (Some(id), _) => format!("dragging: {id}"),
                (None, Some(mv)) => format!(
                    "last move: {} — column {} → {} @ {}",
                    mv.card, mv.from, mv.to, mv.index
                ),
                (None, None) => "drag a card to move it".to_owned(),
            };
            ui.label(
                egui::RichText::new(readout)
                    .font(t.mono(t.type_scale.sm))
                    .color(t.fg[2]),
            );
        });
}
