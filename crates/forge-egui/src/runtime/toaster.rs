//! Toast notifications — mirrors the web `toast()` singleton and forge-tui's
//! Toaster. The handle is `Clone + Send` and carries the `egui::Context` so a
//! push from a background thread wakes eframe's lazy repaint loop.

use crate::theme::{Severity, Theme};
use crate::widgets::Tone;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_VISIBLE: usize = 4;

fn ttl(severity: Severity) -> Duration {
    Duration::from_secs(match severity {
        Severity::Danger => 6,
        Severity::Warning => 5,
        Severity::Success | Severity::Info => 4,
    })
}

struct Toast {
    severity: Severity,
    message: String,
}

/// Push toasts from anywhere: `handle.success("saved")`. Cheap to clone;
/// every push requests a repaint so the toast appears immediately even when
/// the app is idle.
#[derive(Clone)]
pub struct ToastHandle {
    tx: mpsc::Sender<Toast>,
    egui: egui::Context,
}

impl ToastHandle {
    pub fn push(&self, severity: Severity, message: impl Into<String>) {
        let _ = self.tx.send(Toast {
            severity,
            message: message.into(),
        });
        self.egui.request_repaint();
    }

    pub fn info(&self, message: impl Into<String>) {
        self.push(Severity::Info, message);
    }

    pub fn success(&self, message: impl Into<String>) {
        self.push(Severity::Success, message);
    }

    pub fn warning(&self, message: impl Into<String>) {
        self.push(Severity::Warning, message);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.push(Severity::Danger, message);
    }
}

struct ActiveToast {
    toast: Toast,
    expires: Instant,
}

pub(crate) struct Toaster {
    tx: mpsc::Sender<Toast>,
    rx: mpsc::Receiver<Toast>,
    egui: egui::Context,
    active: Vec<ActiveToast>,
}

impl Toaster {
    pub(crate) fn new(egui: egui::Context) -> Toaster {
        let (tx, rx) = mpsc::channel();
        Toaster {
            tx,
            rx,
            egui,
            active: Vec::new(),
        }
    }

    pub(crate) fn handle(&self) -> ToastHandle {
        ToastHandle {
            tx: self.tx.clone(),
            egui: self.egui.clone(),
        }
    }

    /// Drain, expire, and paint the stack (newest on top, top-right).
    pub(crate) fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        let now = Instant::now();
        while let Ok(toast) = self.rx.try_recv() {
            let expires = now + ttl(toast.severity);
            self.active.push(ActiveToast { toast, expires });
        }
        self.active.retain(|t| t.expires > now);
        if self.active.is_empty() {
            return;
        }

        egui::Area::new(egui::Id::new("forge-toasts"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 16.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                for entry in self.active.iter().rev().take(MAX_VISIBLE) {
                    let tone = match entry.toast.severity {
                        Severity::Success => Tone::Success,
                        Severity::Warning => Tone::Warning,
                        Severity::Danger => Tone::Danger,
                        Severity::Info => Tone::Info,
                    };
                    let (base, _, _) = tone.triple(theme);
                    egui::Frame::new()
                        .fill(theme.bg[4])
                        .stroke(egui::Stroke::new(1.0, theme.border.default))
                        .corner_radius(egui::CornerRadius::same(theme.radius.md as u8))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let glyph = match entry.toast.severity {
                                    Severity::Success => "✓",
                                    Severity::Warning => "⚠",
                                    Severity::Danger => "✗",
                                    Severity::Info => "ℹ",
                                };
                                ui.label(egui::RichText::new(glyph).color(base));
                                ui.label(
                                    egui::RichText::new(&entry.toast.message).color(theme.fg[0]),
                                );
                            });
                        });
                    ui.add_space(6.0);
                }
            });

        // Wake up exactly when the next toast expires.
        if let Some(next) = self.active.iter().map(|t| t.expires).min() {
            ctx.request_repaint_after(next.saturating_duration_since(now));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_clone<T: Send + Clone>() {}

    #[test]
    fn handle_is_send_and_clone() {
        assert_send_clone::<ToastHandle>();
    }

    #[test]
    fn push_from_background_thread_lands_in_stack() {
        let ctx = egui::Context::default();
        let mut toaster = Toaster::new(ctx.clone());
        let handle = toaster.handle();
        std::thread::spawn(move || {
            handle.success("saved from a worker");
            handle.error("boom");
        })
        .join()
        .unwrap();

        let theme = Theme::dark();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            toaster.show(&ui.ctx().clone(), &theme);
        });
        assert_eq!(toaster.active.len(), 2);
        assert_eq!(toaster.active[0].toast.message, "saved from a worker");
        // Danger toasts outlive info toasts (6s vs 4s TTL).
        assert!(toaster.active[1].expires > toaster.active[0].expires);
    }
}
