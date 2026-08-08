//! A snapshot of the terminal-capability environment variables, read once at
//! the edge and passed into the decisions that need them.
//!
//! Neither [`ColorMode::detect`](crate::theme::ColorMode::detect) nor
//! [`Motion::resolve`](crate::runtime::Motion::resolve) touches the process
//! environment: both take a [`TermEnv`] value. The runtime captures one at
//! the top of [`run`](crate::runtime::run); tests build values by hand, which
//! is what lets them run in parallel without an environment lock.

/// The environment variables that drive color-mode detection and motion
/// resolution, captured as plain data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TermEnv {
    /// `FORGE_TUI_MOTION` — motion override (`full` / `reduced` / `off`).
    pub motion: Option<String>,
    /// `FORGE_TUI_COLOR` — color-mode override (`truecolor` / `256` / `16`).
    pub color: Option<String>,
    /// `COLORTERM` — how the terminal advertises truecolor support.
    pub colorterm: Option<String>,
    /// `TERM` — the terminal type.
    pub term: Option<String>,
    /// `NO_COLOR` is set to a non-empty value.
    pub no_color: bool,
}

impl TermEnv {
    /// Read the process environment — the one place these variables are
    /// consulted. [`run`](crate::runtime::run) calls this at startup; call it
    /// yourself when driving the resolvers in your own loop.
    pub fn from_process() -> TermEnv {
        TermEnv {
            motion: std::env::var("FORGE_TUI_MOTION").ok(),
            color: std::env::var("FORGE_TUI_COLOR").ok(),
            colorterm: std::env::var("COLORTERM").ok(),
            term: std::env::var("TERM").ok(),
            no_color: std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty()),
        }
    }
}
