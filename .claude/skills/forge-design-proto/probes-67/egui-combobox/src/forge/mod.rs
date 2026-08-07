//! Forge on egui.
//!
//! Forge ships as guidance, not as a package: every type here is written from
//! `.claude/skills/forge-design`. Names come from the catalogue in `SKILL.md`,
//! the grammar in `reference/egui.md`, and the tokens in `reference/tokens.md`.

pub mod combobox;
#[cfg(test)]
mod combobox_test;
pub mod icon;
pub mod response;
pub mod shell;
pub mod theme;

pub use combobox::{ComboBox, ComboBoxOption, ComboBoxState};
pub use response::{ForgeResponse, Outcome};
pub use shell::{AppShell, PageHead, SettingsLayout, SettingsRow, SettingsSection};
pub use theme::{FontWeight, Severity, Theme};
