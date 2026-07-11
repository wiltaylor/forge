//! Terminal page: an auto-started local shell plus an SSH connect form —
//! both forge-core engines running in-process on the backend's runtime
//! (injected via `forge_egui::rt::set_handle` in `Demo::new`).

use forge_egui::prelude::*;
use forge_egui::widgets::{SshOptions, TermState, TermStatus};

pub struct TermPage {
    local: Option<TermState>,
    ssh: Option<TermState>,
    host: String,
    port: String,
    username: String,
    password: String,
}

impl Default for TermPage {
    fn default() -> Self {
        TermPage {
            local: None,
            ssh: None,
            host: "127.0.0.1".to_owned(),
            port: "22".to_owned(),
            username: String::new(),
            password: String::new(),
        }
    }
}

impl TermPage {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let t = Theme::of(ui.ctx());

        Card::new().title("Local shell").show(ui, |ui| {
            let term = self.local.get_or_insert_with(|| TermState::local(ui.ctx()));
            let _ = Terminal::new().rows(24).show(ui, term);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                status_badge(ui, term.status());
                if matches!(
                    term.status(),
                    TermStatus::Exited(_) | TermStatus::Error(_) | TermStatus::Closed
                ) && Button::new("Restart").small(true).show(ui).clicked()
                {
                    term.restart(ui.ctx());
                }
                ui.label(
                    egui::RichText::new("click to capture · Ctrl+Shift+Q releases")
                        .font(t.mono(t.type_scale.xs))
                        .color(t.fg[2]),
                );
            });
        });
        ui.add_space(12.0);

        Card::new().title("SSH").show(ui, |ui| {
            ui.horizontal(|ui| {
                let _ = Input::new(&mut self.host)
                    .placeholder("host")
                    .desired_width(160.0)
                    .show(ui);
                let _ = Input::new(&mut self.port)
                    .placeholder("22")
                    .desired_width(60.0)
                    .show(ui);
                let _ = Input::new(&mut self.username)
                    .placeholder("username")
                    .desired_width(120.0)
                    .show(ui);
                let _ = Input::new(&mut self.password)
                    .placeholder("password")
                    .masked(true)
                    .desired_width(120.0)
                    .show(ui);
                let connectable = !self.host.is_empty() && !self.username.is_empty();
                if self.ssh.is_none() {
                    if Button::new("Connect")
                        .variant(Variant::Primary)
                        .small(true)
                        .disabled(!connectable)
                        .show(ui)
                        .clicked()
                    {
                        let opts = SshOptions {
                            host: self.host.clone(),
                            port: self.port.trim().parse().unwrap_or(22),
                            username: self.username.clone(),
                            password: self.password.clone(),
                        };
                        self.ssh = Some(TermState::ssh(ui.ctx(), opts));
                    }
                } else if Button::new("Disconnect")
                    .variant(Variant::Danger)
                    .small(true)
                    .show(ui)
                    .clicked()
                {
                    if let Some(term) = &mut self.ssh {
                        term.disconnect();
                    }
                    self.ssh = None;
                }
            });

            if let Some(term) = &mut self.ssh {
                ui.add_space(8.0);
                let _ = Terminal::new().rows(24).show(ui, term);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    status_badge(ui, term.status());
                    if matches!(
                        term.status(),
                        TermStatus::Exited(_) | TermStatus::Error(_) | TermStatus::Closed
                    ) && Button::new("Reconnect").small(true).show(ui).clicked()
                    {
                        term.restart(ui.ctx());
                    }
                });
            }
        });
    }
}

fn status_badge(ui: &mut egui::Ui, status: &TermStatus) {
    use forge_egui::widgets::Tone;
    let (label, tone) = match status {
        TermStatus::Connecting => ("connecting…".to_owned(), Tone::Info),
        TermStatus::Ready => ("ready".to_owned(), Tone::Success),
        TermStatus::Exited(code) => (format!("exited (code {code})"), Tone::Warning),
        TermStatus::Error(message) => (format!("error — {message}"), Tone::Danger),
        TermStatus::Closed => ("closed".to_owned(), Tone::Neutral),
    };
    let _ = Badge::new(&label).tone(tone).show(ui);
}
