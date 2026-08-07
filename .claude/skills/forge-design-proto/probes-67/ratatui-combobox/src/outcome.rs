//! The one interaction result every Forge ratatui control returns.
//!
//! `reference/ratatui.md`: the state type owns `handle_key` and returns an `Outcome`.

/// What a control did with a key or a mouse event.
///
/// `Ignored` is what makes key routing composable: a parent keeps offering the event to
/// the next handler until something returns anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not for this widget — keep routing, like DOM bubbling.
    Ignored,
    /// Handled, no observable value change (a cursor move).
    Consumed,
    /// The value or the selection changed.
    Changed,
    /// Enter-style commit — read the value from the state.
    Submitted,
    /// Esc-style dismissal.
    Cancelled,
}

impl Outcome {
    /// Anything but `Ignored`.
    pub fn is_handled(self) -> bool {
        self != Outcome::Ignored
    }
}
