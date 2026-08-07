//! The one key-routing result every Forge ratatui control returns.
//!
//! `reference/ratatui.md`, "Interaction".

/// What a control did with a key.
///
/// `Ignored` is what makes routing composable: a parent keeps offering the key
/// to the next handler until something returns anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not for this widget — keep routing, like DOM bubbling.
    Ignored,
    /// Handled, no observable value change (a cursor move).
    Consumed,
    /// The value or selection changed.
    Changed,
    /// Enter-style commit — read the value from the state.
    Submitted,
    /// Esc-style dismissal.
    Cancelled,
}

impl Outcome {
    /// Anything but [`Outcome::Ignored`].
    pub fn is_handled(self) -> bool {
        !matches!(self, Outcome::Ignored)
    }
}
