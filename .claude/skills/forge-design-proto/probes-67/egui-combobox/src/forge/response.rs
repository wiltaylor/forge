//! What every Forge control returns from `.show(ui)`.
//!
//! `reference/egui.md`: `.show(ui)` returns a `ForgeResponse`, never a bare
//! `egui::Response`.

/// The five outcomes. The same variants as ratatui, and the same meanings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No interaction this frame.
    Ignored,
    /// Interacted — hovered popup, focus moved, opened — no value change.
    Consumed,
    /// The value or the selection changed.
    Changed,
    /// Enter-style commit, or a button activation.
    Submitted,
    /// Esc-style dismissal — dropdown closed, dialog cancelled.
    Cancelled,
}

impl Outcome {
    /// Anything but `Ignored`.
    pub fn is_handled(self) -> bool {
        self != Self::Ignored
    }

    /// The more significant of two outcomes, for a control built out of parts.
    pub fn merge(self, other: Self) -> Self {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Ignored => 0,
            Self::Consumed => 1,
            Self::Changed => 2,
            Self::Cancelled => 3,
            Self::Submitted => 4,
        }
    }
}

/// egui's own response, plus the Forge outcome.
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

    /// Keep this response, and take the more significant of the two outcomes.
    pub fn merge(mut self, other: Outcome) -> Self {
        self.outcome = self.outcome.merge(other);
        self
    }
}
