use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, StatefulWidget, Widget};

/// One kanban column: title + card labels (+ optional WIP limit).
#[derive(Clone, Copy, Debug)]
pub struct KanbanColumn<'a> {
    pub title: &'a str,
    pub cards: &'a [&'a str],
    pub wip_limit: Option<usize>,
}

impl<'a> KanbanColumn<'a> {
    pub fn new(title: &'a str, cards: &'a [&'a str]) -> KanbanColumn<'a> {
        KanbanColumn {
            title,
            cards,
            wip_limit: None,
        }
    }

    pub fn wip_limit(mut self, limit: usize) -> Self {
        self.wip_limit = Some(limit);
        self
    }
}

/// A requested card move — the widget cannot mutate your board, so
/// Shift+arrow moves surface here; apply it to your data and re-render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KanbanMove {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

/// Cursor over `(column, card)` plus the pending move request.
#[derive(Clone, Debug, Default)]
pub struct KanbanState {
    pub col: usize,
    pub card: usize,
    pending: Option<KanbanMove>,
    lens: Vec<usize>,
    card_rects: Vec<(Rect, usize, usize)>,
}

impl KanbanState {
    pub fn new() -> KanbanState {
        KanbanState::default()
    }

    /// The move requested by the last Shift+arrow, if any.
    pub fn take_move(&mut self) -> Option<KanbanMove> {
        self.pending.take()
    }

    /// Click a card to move the cursor; clicking the cursor card submits it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        for (rect, col, card) in self.card_rects.clone() {
            if clicked(ev, rect) {
                if self.col == col && self.card == card {
                    return Outcome::Submitted;
                }
                self.col = col;
                self.card = card;
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }

    fn clamp(&mut self) {
        if self.lens.is_empty() {
            return;
        }
        self.col = self.col.min(self.lens.len() - 1);
        self.card = self.card.min(self.lens[self.col].saturating_sub(1));
    }

    /// ←/→ column, ↑/↓ card, Shift+arrows request a move, Enter submits the
    /// cursor card.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) || self.lens.is_empty() {
            return Outcome::Ignored;
        }
        self.clamp();
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Left if shift => {
                if self.col > 0 && self.lens[self.col] > 0 {
                    let to_col = self.col - 1;
                    self.pending = Some(KanbanMove {
                        from: (self.col, self.card),
                        to: (to_col, self.lens[to_col]),
                    });
                    self.col = to_col;
                    self.card = self.lens[to_col]; // lands where it arrives
                    return Outcome::Changed;
                }
                Outcome::Consumed
            }
            KeyCode::Right if shift => {
                if self.col + 1 < self.lens.len() && self.lens[self.col] > 0 {
                    let to_col = self.col + 1;
                    self.pending = Some(KanbanMove {
                        from: (self.col, self.card),
                        to: (to_col, self.lens[to_col]),
                    });
                    self.col = to_col;
                    self.card = self.lens[to_col];
                    return Outcome::Changed;
                }
                Outcome::Consumed
            }
            KeyCode::Up if shift => {
                if self.card > 0 {
                    self.pending = Some(KanbanMove {
                        from: (self.col, self.card),
                        to: (self.col, self.card - 1),
                    });
                    self.card -= 1;
                    return Outcome::Changed;
                }
                Outcome::Consumed
            }
            KeyCode::Down if shift => {
                if self.lens[self.col] > 0 && self.card + 1 < self.lens[self.col] {
                    self.pending = Some(KanbanMove {
                        from: (self.col, self.card),
                        to: (self.col, self.card + 1),
                    });
                    self.card += 1;
                    return Outcome::Changed;
                }
                Outcome::Consumed
            }
            KeyCode::Left => {
                self.col = self.col.saturating_sub(1);
                self.clamp();
                Outcome::Consumed
            }
            KeyCode::Right => {
                self.col = (self.col + 1).min(self.lens.len() - 1);
                self.clamp();
                Outcome::Consumed
            }
            KeyCode::Up => {
                self.card = self.card.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.card = (self.card + 1).min(self.lens[self.col].saturating_sub(1));
                Outcome::Consumed
            }
            KeyCode::Enter => Outcome::Submitted,
            _ => Outcome::Ignored,
        }
    }
}

/// The board: equal-width columns with counts/WIP badges and 3-row cards;
/// the cursor card gets an accent border.
#[derive(Clone, Debug)]
pub struct Kanban<'a> {
    columns: &'a [KanbanColumn<'a>],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Kanban<'a> {
    pub fn new(columns: &'a [KanbanColumn<'a>]) -> Kanban<'a> {
        Kanban {
            columns,
            focused: false,
            theme: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Kanban<'a> {
    type State = KanbanState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut KanbanState) {
        state.lens = self.columns.iter().map(|c| c.cards.len()).collect();
        state.card_rects.clear();
        state.clamp();
        if area.is_empty() || self.columns.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let n = self.columns.len() as u16;
        let gap = 1u16;
        let col_w = (area.width.saturating_sub(gap * (n - 1))) / n;
        if col_w < 6 {
            return;
        }
        for (ci, column) in self.columns.iter().enumerate() {
            let x = area.x + ci as u16 * (col_w + gap);
            let col_area = Rect::new(x, area.y, col_w, area.height);
            buf.set_style(col_area, Style::new().bg(t.bg[1]));
            // Header: title + count (+ WIP badge when over).
            let over = column.wip_limit.is_some_and(|l| column.cards.len() > l);
            let count = match column.wip_limit {
                Some(l) => format!("{}/{}", column.cards.len(), l),
                None => column.cards.len().to_string(),
            };
            let active_col = ci == state.col;
            buf.set_string(
                x + 1,
                area.y,
                text::truncate(column.title, col_w.saturating_sub(2) as usize),
                Style::new()
                    .fg(if active_col { t.fg[0] } else { t.fg[1] })
                    .bg(t.bg[1])
                    .add_modifier(Modifier::BOLD),
            );
            let cw = text::width(&count) as u16;
            if col_w > cw + 2 {
                buf.set_string(
                    x + col_w - cw - 1,
                    area.y,
                    &count,
                    Style::new()
                        .fg(if over { t.danger.fg } else { t.fg[2] })
                        .bg(t.bg[1]),
                );
            }
            // Cards.
            let mut y = area.y + 1;
            for (idx, card) in column.cards.iter().enumerate() {
                if y + 3 > area.y + area.height {
                    break;
                }
                let card_area = Rect::new(x, y, col_w, 3);
                state.card_rects.push((card_area, ci, idx));
                let is_cursor = active_col && idx == state.card;
                let border = if is_cursor {
                    if self.focused {
                        t.accent.base
                    } else {
                        t.border.strong
                    }
                } else {
                    t.border.default
                };
                let block = Block::bordered()
                    .border_style(Style::new().fg(border).bg(t.bg[2]))
                    .style(Style::new().bg(t.bg[2]));
                let inner = block.inner(card_area);
                block.render(card_area, buf);
                buf.set_string(
                    inner.x + 1,
                    inner.y,
                    text::truncate(card, inner.width.saturating_sub(2) as usize),
                    Style::new()
                        .fg(if is_cursor { t.fg[0] } else { t.fg[1] })
                        .bg(t.bg[2]),
                );
                y += 3;
            }
        }
    }
}
