//! Forge controls for ratatui 0.29.
//!
//! Built from the `forge-design` skill. The crate is the namespace, so a control's type
//! name is its catalogue name in PascalCase with no prefix.
//!
//! Crossterm is imported through `ratatui::crossterm`, never as a direct dependency, so
//! the two versions can never diverge.

pub mod combobox;
pub mod input;
pub mod outcome;
pub mod theme;

pub use combobox::{ComboBox, ComboBoxItem, ComboBoxState};
pub use input::{Input, InputState};
pub use outcome::Outcome;
pub use theme::{AccentColors, BorderColors, Severity, StatusColors, Theme};
