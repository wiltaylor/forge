//! Imperative toasts, mirroring the web's `toast()` singleton. Any thread can
//! push through a cloned [`ToastHandle`]; the runtime drains the queue on its
//! tick and paints the stack in the top-right corner.

use crate::theme::{Severity, Theme};
use crate::widgets::feedback::ToastView;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Toast {
    pub severity: Severity,
    pub message: String,
}

/// Cheap, `Clone + Send` handle for pushing toasts from anywhere.
#[derive(Clone)]
pub struct ToastHandle {
    tx: Sender<Toast>,
}

impl ToastHandle {
    pub fn push(&self, severity: Severity, message: impl Into<String>) {
        let _ = self.tx.send(Toast { severity, message: message.into() });
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

struct Active {
    toast: Toast,
    expires: Instant,
}

pub struct Toaster {
    tx: Sender<Toast>,
    rx: Receiver<Toast>,
    active: Vec<Active>,
    max_visible: usize,
}

impl Default for Toaster {
    fn default() -> Toaster {
        Toaster::new()
    }
}

impl Toaster {
    pub fn new() -> Toaster {
        let (tx, rx) = channel();
        Toaster { tx, rx, active: Vec::new(), max_visible: 4 }
    }

    pub fn handle(&self) -> ToastHandle {
        ToastHandle { tx: self.tx.clone() }
    }

    fn ttl(severity: Severity) -> Duration {
        match severity {
            Severity::Danger => Duration::from_secs(6),
            Severity::Warning => Duration::from_secs(5),
            _ => Duration::from_secs(4),
        }
    }

    /// Drain queued toasts and expire old ones. Called by the runtime tick.
    pub fn tick(&mut self) {
        let now = Instant::now();
        while let Ok(toast) = self.rx.try_recv() {
            let expires = now + Toaster::ttl(toast.severity);
            self.active.push(Active { toast, expires });
        }
        self.active.retain(|a| a.expires > now);
    }

    /// Dismiss the oldest visible toast (bind to a key if desired).
    pub fn dismiss(&mut self) {
        if !self.active.is_empty() {
            self.active.remove(0);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// Paint the newest `max_visible` toasts stacked from the top-right.
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut y = area.y + 1;
        let start = self.active.len().saturating_sub(self.max_visible);
        for active in &self.active[start..] {
            let view = ToastView::new(active.toast.severity, &active.toast.message).theme(theme);
            let (w, h) = view.size(area.width.saturating_sub(4).min(44));
            if y + h > area.y + area.height {
                break;
            }
            let x = area.x + area.width.saturating_sub(w + 2);
            frame.render_widget(view, Rect::new(x, y, w, h));
            y += h;
        }
    }
}
