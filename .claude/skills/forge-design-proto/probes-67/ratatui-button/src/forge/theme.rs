//! Forge theme for ratatui.
//!
//! `reference/tokens.md` names every field here. The geometry, type and motion
//! columns of that page are empty for ratatui on purpose, so this struct carries
//! colour only — a terminal has cells, not pixels.
//!
//! `reference/impl/ratatui/theme-provider.md` is "Not written", so the concrete
//! channel values below were chosen here, not read from the skill.

use ratatui::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// `theme.border.*` — the 1px division roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderColors {
    pub default: Color,
    pub subtle: Color,
    pub strong: Color,
}

/// `theme.accent.*` — one accent per screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    pub base: Color,
    pub hover: Color,
    pub press: Color,
    /// The 14% tint.
    pub bg: Color,
    /// Accent as a text colour.
    pub fg: Color,
    /// Text that goes on the solid accent.
    pub contrast: Color,
}

/// `theme.success` / `warning` / `danger` / `info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusColors {
    /// The solid.
    pub base: Color,
    /// The 14% tint.
    pub bg: Color,
    /// Text that goes on that tint.
    pub fg: Color,
}

/// The four statuses, reachable through [`Theme::severity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Success,
    Warning,
    Danger,
    Info,
}

/// The Forge palette. Pass it into a control with `.theme(&theme)`; a control
/// never reaches for it globally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// `bg[0]` surface, `bg[1]` raised, `bg[2]` hover, `bg[3]` pressed, `bg[4]` popover.
    pub bg: [Color; 5],
    /// `fg[0]` text, `fg[1]` secondary, `fg[2]` dim, `fg[3]` disabled.
    pub fg: [Color; 4],
    pub border: BorderColors,
    pub accent: AccentColors,
    pub success: StatusColors,
    pub warning: StatusColors,
    pub danger: StatusColors,
    pub info: StatusColors,
}

impl Theme {
    /// The default theme. Forge is dark by default.
    pub const fn dark() -> Self {
        Self {
            bg: [
                rgb(0x0d, 0x11, 0x17),
                rgb(0x16, 0x1b, 0x22),
                rgb(0x21, 0x26, 0x2d),
                rgb(0x2d, 0x33, 0x3b),
                rgb(0x1c, 0x21, 0x28),
            ],
            fg: [
                rgb(0xe6, 0xed, 0xf3),
                rgb(0xad, 0xba, 0xc7),
                rgb(0x76, 0x83, 0x90),
                rgb(0x54, 0x5d, 0x68),
            ],
            border: BorderColors {
                default: rgb(0x30, 0x36, 0x3d),
                subtle: rgb(0x21, 0x26, 0x2d),
                strong: rgb(0x44, 0x4c, 0x56),
            },
            accent: AccentColors {
                base: rgb(0x2f, 0x81, 0xf7),
                hover: rgb(0x4a, 0x94, 0xf8),
                press: rgb(0x1f, 0x6f, 0xeb),
                bg: rgb(0x10, 0x23, 0x3f),
                fg: rgb(0x58, 0xa6, 0xff),
                contrast: rgb(0xff, 0xff, 0xff),
            },
            success: StatusColors {
                base: rgb(0x3f, 0xb9, 0x50),
                bg: rgb(0x0f, 0x2a, 0x17),
                fg: rgb(0x56, 0xd3, 0x64),
            },
            warning: StatusColors {
                base: rgb(0xd2, 0x99, 0x22),
                bg: rgb(0x2b, 0x23, 0x08),
                fg: rgb(0xe3, 0xb3, 0x41),
            },
            danger: StatusColors {
                base: rgb(0xf8, 0x51, 0x49),
                bg: rgb(0x3a, 0x14, 0x16),
                fg: rgb(0xff, 0x7b, 0x72),
            },
            info: StatusColors {
                base: rgb(0x58, 0xa6, 0xff),
                bg: rgb(0x0d, 0x24, 0x40),
                fg: rgb(0x79, 0xc0, 0xff),
            },
        }
    }

    /// The light theme. Forge defines one on every platform; never ship dark-only.
    pub const fn light() -> Self {
        Self {
            bg: [
                rgb(0xff, 0xff, 0xff),
                rgb(0xf6, 0xf8, 0xfa),
                rgb(0xea, 0xee, 0xf2),
                rgb(0xd0, 0xd7, 0xde),
                rgb(0xff, 0xff, 0xff),
            ],
            fg: [
                rgb(0x1f, 0x23, 0x28),
                rgb(0x42, 0x4a, 0x53),
                rgb(0x65, 0x6d, 0x76),
                rgb(0x8c, 0x95, 0x9f),
            ],
            border: BorderColors {
                default: rgb(0xd0, 0xd7, 0xde),
                subtle: rgb(0xea, 0xee, 0xf2),
                strong: rgb(0xaf, 0xb8, 0xc1),
            },
            accent: AccentColors {
                base: rgb(0x09, 0x69, 0xda),
                hover: rgb(0x21, 0x8b, 0xff),
                press: rgb(0x05, 0x50, 0xae),
                bg: rgb(0xdd, 0xf4, 0xff),
                fg: rgb(0x09, 0x69, 0xda),
                contrast: rgb(0xff, 0xff, 0xff),
            },
            success: StatusColors {
                base: rgb(0x1a, 0x7f, 0x37),
                bg: rgb(0xda, 0xfb, 0xe1),
                fg: rgb(0x11, 0x63, 0x29),
            },
            warning: StatusColors {
                base: rgb(0x9a, 0x67, 0x00),
                bg: rgb(0xff, 0xf8, 0xc5),
                fg: rgb(0x7d, 0x4e, 0x00),
            },
            danger: StatusColors {
                base: rgb(0xcf, 0x22, 0x2e),
                bg: rgb(0xff, 0xeb, 0xe9),
                fg: rgb(0xa4, 0x0e, 0x26),
            },
            info: StatusColors {
                base: rgb(0x09, 0x69, 0xda),
                bg: rgb(0xdd, 0xf4, 0xff),
                fg: rgb(0x05, 0x50, 0xae),
            },
        }
    }

    /// Reach a status triple by severity.
    pub const fn severity(&self, severity: Severity) -> StatusColors {
        match severity {
            Severity::Success => self.success,
            Severity::Warning => self.warning,
            Severity::Danger => self.danger,
            Severity::Info => self.info,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
