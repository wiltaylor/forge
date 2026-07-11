//! Desktop page: VNC + RDP connect forms driving the remote-desktop viewer —
//! forge-core engines running in-process on the backend's runtime (injected
//! via `forge_egui::rt::set_handle` in `Demo::new`). The prefilled targets
//! match `just widgets-testenv-up` (VNC :5900 password "forge", RDP :3389
//! forge/forge).

use forge_egui::prelude::*;
use forge_egui::widgets::{
    DesktopState, DesktopStatus, DesktopView, RdpTarget, ScaleMode, VncTarget,
};

const SCALE_LABELS: &[&str] = &["Fit", "1:1", "Stretch"];

fn scale_mode(index: usize) -> ScaleMode {
    match index {
        1 => ScaleMode::OneToOne,
        2 => ScaleMode::Stretch,
        _ => ScaleMode::Fit,
    }
}

pub struct DesktopPage {
    vnc: Option<DesktopState>,
    vnc_host: String,
    vnc_port: String,
    vnc_password: String,
    vnc_scale: usize,
    rdp: Option<DesktopState>,
    rdp_host: String,
    rdp_port: String,
    rdp_username: String,
    rdp_password: String,
    rdp_scale: usize,
}

impl Default for DesktopPage {
    fn default() -> Self {
        DesktopPage {
            vnc: None,
            vnc_host: "127.0.0.1".to_owned(),
            vnc_port: "5900".to_owned(),
            vnc_password: "forge".to_owned(),
            vnc_scale: 0,
            rdp: None,
            rdp_host: "127.0.0.1".to_owned(),
            rdp_port: "3389".to_owned(),
            rdp_username: "forge".to_owned(),
            rdp_password: "forge".to_owned(),
            rdp_scale: 0,
        }
    }
}

impl DesktopPage {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Dev hook: FORGE_DEMO_AUTOCONNECT=vnc|rdp connects the prefilled
        // form on first show (pairs with FORGE_DEMO_SHOT for headless
        // verification against `just widgets-testenv-up`).
        match std::env::var("FORGE_DEMO_AUTOCONNECT").as_deref() {
            Ok("vnc") if self.vnc.is_none() => {
                let target = VncTarget {
                    host: self.vnc_host.clone(),
                    port: self.vnc_port.trim().parse().unwrap_or(5900),
                    password: (!self.vnc_password.is_empty())
                        .then(|| self.vnc_password.clone()),
                };
                self.vnc = Some(DesktopState::vnc(ui.ctx(), target));
            }
            Ok("rdp") if self.rdp.is_none() => {
                let target = RdpTarget {
                    host: self.rdp_host.clone(),
                    port: self.rdp_port.trim().parse().unwrap_or(3389),
                    username: self.rdp_username.clone(),
                    password: self.rdp_password.clone(),
                };
                self.rdp = Some(DesktopState::rdp(ui.ctx(), target));
            }
            _ => {}
        }

        Card::new().title("VNC").show(ui, |ui| {
            ui.horizontal(|ui| {
                let _ = Input::new(&mut self.vnc_host)
                    .placeholder("host")
                    .desired_width(160.0)
                    .show(ui);
                let _ = Input::new(&mut self.vnc_port)
                    .placeholder("5900")
                    .desired_width(60.0)
                    .show(ui);
                let _ = Input::new(&mut self.vnc_password)
                    .placeholder("password")
                    .masked(true)
                    .desired_width(120.0)
                    .show(ui);
                if self.vnc.is_none() {
                    if Button::new("Connect")
                        .variant(Variant::Primary)
                        .small(true)
                        .disabled(self.vnc_host.is_empty())
                        .show(ui)
                        .clicked()
                    {
                        let target = VncTarget {
                            host: self.vnc_host.clone(),
                            port: self.vnc_port.trim().parse().unwrap_or(5900),
                            password: (!self.vnc_password.is_empty())
                                .then(|| self.vnc_password.clone()),
                        };
                        self.vnc = Some(DesktopState::vnc(ui.ctx(), target));
                    }
                } else if Button::new("Disconnect")
                    .variant(Variant::Danger)
                    .small(true)
                    .show(ui)
                    .clicked()
                {
                    if let Some(desk) = &mut self.vnc {
                        desk.disconnect();
                    }
                    self.vnc = None;
                }
            });

            if let Some(desk) = &mut self.vnc {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let _ = ToggleGroup::new(&mut self.vnc_scale, SCALE_LABELS).show(ui);
                    status_badge(ui, desk.status(), desk.fb_size());
                });
                ui.add_space(6.0);
                let _ = DesktopView::new()
                    .scale(scale_mode(self.vnc_scale))
                    .show(ui, desk);
            }
        });
        ui.add_space(12.0);

        Card::new().title("RDP").show(ui, |ui| {
            ui.horizontal(|ui| {
                let _ = Input::new(&mut self.rdp_host)
                    .placeholder("host")
                    .desired_width(160.0)
                    .show(ui);
                let _ = Input::new(&mut self.rdp_port)
                    .placeholder("3389")
                    .desired_width(60.0)
                    .show(ui);
                let _ = Input::new(&mut self.rdp_username)
                    .placeholder("DOMAIN\\user")
                    .desired_width(120.0)
                    .show(ui);
                let _ = Input::new(&mut self.rdp_password)
                    .placeholder("password")
                    .masked(true)
                    .desired_width(120.0)
                    .show(ui);
                let connectable = !self.rdp_host.is_empty() && !self.rdp_username.is_empty();
                if self.rdp.is_none() {
                    if Button::new("Connect")
                        .variant(Variant::Primary)
                        .small(true)
                        .disabled(!connectable)
                        .show(ui)
                        .clicked()
                    {
                        let mut target = RdpTarget::new(
                            self.rdp_host.clone(),
                            self.rdp_username.clone(),
                            self.rdp_password.clone(),
                        );
                        target.port = self.rdp_port.trim().parse().unwrap_or(3389);
                        self.rdp = Some(DesktopState::rdp(ui.ctx(), target));
                    }
                } else if Button::new("Disconnect")
                    .variant(Variant::Danger)
                    .small(true)
                    .show(ui)
                    .clicked()
                {
                    if let Some(desk) = &mut self.rdp {
                        desk.disconnect();
                    }
                    self.rdp = None;
                }
            });

            if let Some(desk) = &mut self.rdp {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let _ = ToggleGroup::new(&mut self.rdp_scale, SCALE_LABELS).show(ui);
                    if Button::new("Ctrl+Alt+Del")
                        .small(true)
                        .disabled(*desk.status() != DesktopStatus::Ready)
                        .show(ui)
                        .clicked()
                    {
                        desk.send_cad();
                    }
                    status_badge(ui, desk.status(), desk.fb_size());
                });
                ui.add_space(6.0);
                let _ = DesktopView::new()
                    .scale(scale_mode(self.rdp_scale))
                    .show(ui, desk);
            }
        });
    }
}

fn status_badge(ui: &mut egui::Ui, status: &DesktopStatus, fb: Option<(u16, u16)>) {
    use forge_egui::widgets::Tone;
    let (label, tone) = match status {
        DesktopStatus::Connecting => ("connecting…".to_owned(), Tone::Info),
        DesktopStatus::Ready => match fb {
            Some((w, h)) => (format!("ready · {w}×{h}"), Tone::Success),
            None => ("ready".to_owned(), Tone::Success),
        },
        DesktopStatus::Error(message) => (format!("error — {message}"), Tone::Danger),
        DesktopStatus::Closed => ("closed".to_owned(), Tone::Neutral),
    };
    let _ = Badge::new(&label).tone(tone).show(ui);
}
