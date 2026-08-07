//! `ForgeResponse` and `Outcome` — the return of every `.show(ui)`.
//!
//! Source: `reference/egui.md`, section "Interaction". The five variants and their
//! meanings are shared with the ratatui kit, deliberately.

use std::ops::Deref;

/// What a control did this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// No interaction this frame.
    #[default]
    Ignored,
    /// Interacted — hovered popup, focus moved, opened — no value change.
    Consumed,
    /// The value or selection changed.
    Changed,
    /// Enter-style commit, or a button activation.
    Submitted,
    /// Esc-style dismissal — dropdown closed, dialog cancelled.
    Cancelled,
}

impl Outcome {
    /// Anything but `Ignored`.
    pub fn is_handled(self) -> bool {
        self != Outcome::Ignored
    }

    /// The more significant of two outcomes, for a control built out of parts.
    pub fn merge(self, other: Outcome) -> Outcome {
        if other > self {
            other
        } else {
            self
        }
    }
}

/// egui's own response, plus the Forge `Outcome`.
#[derive(Clone, Debug)]
pub struct ForgeResponse {
    pub response: egui::Response,
    pub outcome: Outcome,
}

impl ForgeResponse {
    pub fn new(response: egui::Response, outcome: Outcome) -> Self {
        Self { response, outcome }
    }

    /// Anything but `Ignored`.
    pub fn is_handled(&self) -> bool {
        self.outcome.is_handled()
    }

    /// The control was activated — a button click, or an Enter-style commit.
    pub fn submitted(&self) -> bool {
        self.outcome == Outcome::Submitted
    }

    /// The value or the selection changed.
    pub fn changed(&self) -> bool {
        self.outcome == Outcome::Changed
    }

    /// The control was dismissed.
    pub fn cancelled(&self) -> bool {
        self.outcome == Outcome::Cancelled
    }

    /// The more significant of two outcomes, keeping this response's egui half.
    pub fn merge(mut self, other: Outcome) -> Self {
        self.outcome = self.outcome.merge(other);
        self
    }
}

impl Deref for ForgeResponse {
    type Target = egui::Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}
