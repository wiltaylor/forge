//! Terminal: the embedded local-PTY terminal widget (feature `term`).

use forge_egui::prelude::*;
use forge_egui::widgets::{TermState, TermStatus};

#[derive(Default)]
pub struct TermSectionState {
    /// Lazily constructed so opening the gallery never spawns a shell.
    term: Option<TermState>,
}

pub fn draw(ui: &mut egui::Ui, state: &mut TermSectionState) {
    let t = Theme::of(ui.ctx());

    Card::new().title("Local terminal").show(ui, |ui| {
        ui.horizontal(|ui| {
            if state.term.is_none() {
                if Button::new("Start shell")
                    .variant(Variant::Primary)
                    .small(true)
                    .show(ui)
                    .clicked()
                {
                    state.term = Some(TermState::local(ui.ctx()));
                }
            } else {
                if Button::new("Stop")
                    .variant(Variant::Danger)
                    .small(true)
                    .show(ui)
                    .clicked()
                {
                    if let Some(term) = &mut state.term {
                        term.disconnect();
                    }
                    state.term = None;
                }
                if let Some(term) = &mut state.term {
                    if matches!(
                        term.status(),
                        TermStatus::Exited(_) | TermStatus::Error(_) | TermStatus::Closed
                    ) && Button::new("Restart").small(true).show(ui).clicked()
                    {
                        term.restart(ui.ctx());
                    }
                }
            }
        });

        match &mut state.term {
            Some(term) => {
                ui.add_space(8.0);
                let _ = Terminal::new().rows(24).show(ui, term);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(status_line(term.status()))
                        .font(t.mono(t.type_scale.sm))
                        .color(t.fg[2]),
                );
            }
            None => {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Spawns $SHELL on a PTY via the forge-core engine. \
                         Click the well to type; Ctrl+Shift+Q releases the keyboard.",
                    )
                    .color(t.fg[2]),
                );
            }
        }
    });
}

fn status_line(status: &TermStatus) -> String {
    match status {
        TermStatus::Connecting => "status: connecting…".to_owned(),
        TermStatus::Ready => "status: ready".to_owned(),
        TermStatus::Exited(code) => format!("status: exited (code {code})"),
        TermStatus::Error(message) => format!("status: error — {message}"),
        TermStatus::Closed => "status: closed".to_owned(),
    }
}
