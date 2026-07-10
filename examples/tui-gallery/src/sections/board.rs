use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const BOARD: FocusId = FocusId::new("kb-board");

pub struct BoardState {
    pub kanban: KanbanState,
    pub columns: Vec<(String, Vec<String>)>,
}

impl Default for BoardState {
    fn default() -> BoardState {
        BoardState {
            kanban: KanbanState::new(),
            columns: vec![
                (
                    "Backlog".into(),
                    vec!["RDP clipboard".into(), "Chart tooltips".into(), "Kanban swimlanes".into()],
                ),
                (
                    "In progress".into(),
                    vec!["forge-tui phase 3".into(), "Docs pass".into()],
                ),
                ("Review".into(), vec!["Auth session rotation".into()]),
                ("Done".into(), vec!["Terminal widget".into(), "VNC viewer".into()]),
            ],
        }
    }
}

impl BoardState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        if focused != Some(BOARD) {
            return Outcome::Ignored;
        }
        let outcome = self.kanban.handle_key(key);
        if let Some(mv) = self.kanban.take_move() {
            let card = self.columns[mv.from.0].1.remove(mv.from.1);
            let to_col = &mut self.columns[mv.to.0].1;
            let at = mv.to.1.min(to_col.len());
            to_col.insert(at, card);
        }
        if outcome == Outcome::Submitted {
            let card = self
                .columns
                .get(self.kanban.col)
                .and_then(|c| c.1.get(self.kanban.card));
            if let Some(card) = card {
                ctx.toast().info(format!("Open card: {card}"));
            }
            return Outcome::Consumed;
        }
        outcome
    }
}

impl BoardState {
    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        let out = self.kanban.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(BOARD);
            if out == Outcome::Submitted {
                let card = self
                    .columns
                    .get(self.kanban.col)
                    .and_then(|c| c.1.get(self.kanban.card));
                if let Some(card) = card {
                    ctx.toast().info(format!("Open card: {card}"));
                }
                return Outcome::Consumed;
            }
        }
        out
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut BoardState) {
    let focused = ctx.focus.register(BOARD);
    if area.height < 5 {
        return;
    }
    frame.render_widget(
        Eyebrow::new("Kanban — arrows move cursor · Shift+arrows move card").theme(t),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let card_refs: Vec<Vec<&str>> = state
        .columns
        .iter()
        .map(|(_, cards)| cards.iter().map(String::as_str).collect())
        .collect();
    let columns: Vec<KanbanColumn> = state
        .columns
        .iter()
        .zip(&card_refs)
        .enumerate()
        .map(|(i, ((title, _), cards))| {
            let col = KanbanColumn::new(title, cards);
            if i == 1 {
                col.wip_limit(2)
            } else {
                col
            }
        })
        .collect();
    frame.render_stateful_widget(
        Kanban::new(&columns).focused(focused).theme(t),
        Rect::new(area.x, area.y + 1, area.width, area.height - 1),
        &mut state.kanban,
    );
}
