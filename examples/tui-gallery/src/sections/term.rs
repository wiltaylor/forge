use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const TERM: FocusId = FocusId::new("tm-term");

#[derive(Default)]
pub struct TermState {
    pub session: Option<TerminalState>,
}

impl TermState {
    pub fn handle_key(
        &mut self,
        focused: Option<FocusId>,
        key: KeyEvent,
        ctx: &mut Ctx,
    ) -> Outcome {
        if focused != Some(TERM) || !is_press(&key) {
            return Outcome::Ignored;
        }
        match &mut self.session {
            None => {
                if key.code == KeyCode::Enter {
                    match TerminalState::spawn_shell(24, 80) {
                        Ok(session) => self.session = Some(session),
                        Err(e) => ctx.toast().error(format!("PTY spawn failed: {e}")),
                    }
                    Outcome::Consumed
                } else {
                    Outcome::Ignored
                }
            }
            Some(session) => {
                if session.exited() {
                    self.session = None;
                    return Outcome::Consumed;
                }
                session.handle_key(key)
            }
        }
    }

    /// Forward mouse events to the live session. The widget maps coords using
    /// the pane rect it stored on its last render, so no area is needed here;
    /// it also ignores everything unless the running app enabled mouse tracking.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        match &mut self.session {
            Some(session) if !session.exited() => session.handle_mouse(*ev),
            _ => Outcome::Ignored,
        }
    }

    pub fn tick(&mut self) {
        if let Some(session) = &mut self.session {
            session.drain();
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut TermState) {
    let focused = ctx.focus.register(TERM);
    if area.height < 4 {
        return;
    }
    frame.render_widget(
        Eyebrow::new(
            "Terminal — Enter starts $SHELL · mouse works in TUI apps · Tab leaves the pane",
        )
        .theme(t),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let pane = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    match &mut state.session {
        Some(session) if !session.exited() => {
            frame.render_stateful_widget(Terminal::new().focused(focused).theme(t), pane, session);
        }
        _ => {
            frame.render_widget(
                Empty::new("No session")
                    .hint(if focused {
                        "Press Enter to start a shell"
                    } else {
                        "Tab here, then Enter"
                    })
                    .theme(t),
                pane,
            );
        }
    }
}
