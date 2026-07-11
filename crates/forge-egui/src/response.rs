//! The Forge interaction contract, shared by every kit (web, tui, egui).
//!
//! Every widget's `.show(ui)` returns a [`ForgeResponse`]: the full
//! [`egui::Response`] plus an [`Outcome`] saying what the widget *did* this
//! frame. Unlike forge-tui — where the app routes key events and each widget's
//! `handle_key` computes the outcome — egui owns focus, hover, and event
//! routing, so here the outcome is derived from egui interaction. The names
//! and semantics are identical across kits.

/// What the widget did this frame.
#[must_use = "check .changed()/.submitted(), or ignore explicitly"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No interaction this frame.
    Ignored,
    /// Interacted (hovered popup, focus moved, opened) but no value change.
    Consumed,
    /// Value or selection changed.
    Changed,
    /// Enter-style commit or button activation.
    Submitted,
    /// Esc-style dismissal (dropdown closed, dialog cancelled).
    Cancelled,
}

impl Outcome {
    /// Anything other than [`Outcome::Ignored`].
    pub fn is_handled(self) -> bool {
        self != Outcome::Ignored
    }

    /// The more significant of two outcomes — for widgets composed of parts.
    pub fn merge(self, other: Outcome) -> Outcome {
        use Outcome::*;
        match (self, other) {
            (Submitted, _) | (_, Submitted) => Submitted,
            (Cancelled, _) | (_, Cancelled) => Cancelled,
            (Changed, _) | (_, Changed) => Changed,
            (Consumed, _) | (_, Consumed) => Consumed,
            _ => Ignored,
        }
    }
}

/// An [`egui::Response`] plus the Forge [`Outcome`]. Derefs to the response,
/// so all of egui's inspection methods (`hovered`, `rect`, …) stay available.
pub struct ForgeResponse {
    pub response: egui::Response,
    pub outcome: Outcome,
}

impl ForgeResponse {
    pub fn new(response: egui::Response, outcome: Outcome) -> Self {
        Self { response, outcome }
    }

    pub fn changed(&self) -> bool {
        self.outcome == Outcome::Changed
    }

    pub fn submitted(&self) -> bool {
        self.outcome == Outcome::Submitted
    }

    pub fn cancelled(&self) -> bool {
        self.outcome == Outcome::Cancelled
    }

    pub fn clicked(&self) -> bool {
        self.response.clicked()
    }
}

impl std::ops::Deref for ForgeResponse {
    type Target = egui::Response;
    fn deref(&self) -> &egui::Response {
        &self.response
    }
}
