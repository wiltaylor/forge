//! Forge theme for ratatui.
//!
//! `reference/tokens.md` names every field here. The literal colour values live in this
//! file and nowhere else — this *is* the token table. Call sites reference a field, never
//! a literal.
//!
//! `reference/ratatui.md`: the theme is passed in with `.theme(&theme)`, never reached
//! for globally inside a widget.

use ratatui::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// `border`, `border-subtle`, `border-strong`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderColors {
    pub default: Color,
    pub subtle: Color,
    pub strong: Color,
}

/// The accent ramp. `laws.md`: the accent never fills a large area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    pub base: Color,
    pub hover: Color,
    pub press: Color,
    /// 14% tint.
    pub bg: Color,
    /// Accent as a text colour.
    pub fg: Color,
    /// Text that goes on a solid accent.
    pub contrast: Color,
}

/// One status triple. `base` is the solid, `bg` a 14% tint, `fg` the text on that tint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusColors {
    pub base: Color,
    pub bg: Color,
    pub fg: Color,
}

/// The four statuses. `tokens.md` spells them `success` and `warning`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Success,
    Warning,
    Danger,
    Info,
}

/// The Forge palette. Field order mirrors the web `Theme` interface on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// `bg[0]` page, `[1]` raised, `[2]` hover, `[3]` pressed, `[4]` popover.
    pub bg: [Color; 5],
    /// `fg[0]` primary, `[1]` secondary, `[2]` dim, `[3]` disabled / placeholder.
    pub fg: [Color; 4],
    pub border: BorderColors,
    pub accent: AccentColors,
    pub success: StatusColors,
    pub warning: StatusColors,
    pub danger: StatusColors,
    pub info: StatusColors,
}

impl Default for Theme {
    /// Forge is dark by default.
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// The default theme.
    pub fn dark() -> Self {
        Self {
            bg: [
                rgb(0x0b, 0x0d, 0x10),
                rgb(0x13, 0x16, 0x1b),
                rgb(0x1a, 0x1f, 0x26),
                rgb(0x23, 0x29, 0x32),
                rgb(0x1b, 0x21, 0x29),
            ],
            fg: [
                rgb(0xe6, 0xea, 0xf0),
                rgb(0xb8, 0xc0, 0xcc),
                rgb(0x8a, 0x94, 0xa3),
                rgb(0x5a, 0x64, 0x72),
            ],
            border: BorderColors {
                default: rgb(0x2a, 0x31, 0x3b),
                subtle: rgb(0x1e, 0x24, 0x2c),
                strong: rgb(0x3d, 0x46, 0x53),
            },
            accent: AccentColors {
                base: rgb(0x4d, 0x9f, 0xff),
                hover: rgb(0x6b, 0xb0, 0xff),
                press: rgb(0x3a, 0x8a, 0xe6),
                bg: rgb(0x16, 0x28, 0x3d),
                fg: rgb(0x8c, 0xc2, 0xff),
                contrast: rgb(0x06, 0x12, 0x1f),
            },
            success: StatusColors {
                base: rgb(0x3f, 0xb9, 0x50),
                bg: rgb(0x12, 0x2a, 0x17),
                fg: rgb(0x6f, 0xd4, 0x7c),
            },
            warning: StatusColors {
                base: rgb(0xd2, 0x99, 0x22),
                bg: rgb(0x2e, 0x24, 0x10),
                fg: rgb(0xe3, 0xb3, 0x41),
            },
            danger: StatusColors {
                base: rgb(0xf8, 0x51, 0x49),
                bg: rgb(0x33, 0x16, 0x1a),
                fg: rgb(0xff, 0x8a, 0x80),
            },
            info: StatusColors {
                base: rgb(0x58, 0xa6, 0xff),
                bg: rgb(0x12, 0x24, 0x3a),
                fg: rgb(0x79, 0xb8, 0xff),
            },
        }
    }

    /// The light theme. `laws.md`: never ship light-only, and never ship dark-only.
    pub fn light() -> Self {
        Self {
            bg: [
                rgb(0xff, 0xff, 0xff),
                rgb(0xf6, 0xf8, 0xfa),
                rgb(0xee, 0xf1, 0xf5),
                rgb(0xe3, 0xe8, 0xee),
                rgb(0xff, 0xff, 0xff),
            ],
            fg: [
                rgb(0x10, 0x14, 0x1a),
                rgb(0x3a, 0x43, 0x50),
                rgb(0x61, 0x6b, 0x7a),
                rgb(0x98, 0xa1, 0xae),
            ],
            border: BorderColors {
                default: rgb(0xd5, 0xda, 0xe1),
                subtle: rgb(0xe6, 0xea, 0xef),
                strong: rgb(0xb9, 0xc1, 0xcb),
            },
            accent: AccentColors {
                base: rgb(0x0b, 0x62, 0xd6),
                hover: rgb(0x0a, 0x55, 0xba),
                press: rgb(0x08, 0x47, 0x9c),
                bg: rgb(0xe3, 0xed, 0xfc),
                fg: rgb(0x0b, 0x56, 0xbd),
                contrast: rgb(0xff, 0xff, 0xff),
            },
            success: StatusColors {
                base: rgb(0x1a, 0x7f, 0x37),
                bg: rgb(0xe2, 0xf5, 0xe7),
                fg: rgb(0x16, 0x6b, 0x2e),
            },
            warning: StatusColors {
                base: rgb(0x9a, 0x6c, 0x00),
                bg: rgb(0xfa, 0xf0, 0xd8),
                fg: rgb(0x81, 0x5b, 0x00),
            },
            danger: StatusColors {
                base: rgb(0xcf, 0x22, 0x2e),
                bg: rgb(0xfb, 0xe4, 0xe6),
                fg: rgb(0xa9, 0x1b, 0x25),
            },
            info: StatusColors {
                base: rgb(0x0b, 0x62, 0xd6),
                bg: rgb(0xe3, 0xed, 0xfc),
                fg: rgb(0x0b, 0x56, 0xbd),
            },
        }
    }

    /// Derive hover, press, tint and text from one brand colour.
    ///
    /// Only an RGB colour can be derived from. Any other `Color` is taken as-is for the
    /// whole ramp, because there is nothing to interpolate.
    pub fn with_accent(mut self, base: Color) -> Self {
        let Color::Rgb(r, g, b) = base else {
            self.accent = AccentColors {
                base,
                hover: base,
                press: base,
                bg: self.accent.bg,
                fg: base,
                contrast: self.accent.contrast,
            };
            return self;
        };
        let page = self.bg[0];
        self.accent = AccentColors {
            base,
            hover: mix(base, rgb(0xff, 0xff, 0xff), 0.18),
            press: mix(base, rgb(0x00, 0x00, 0x00), 0.14),
            bg: mix(page, base, 0.14),
            fg: mix(base, self.fg[0], 0.30),
            contrast: if luminance(r, g, b) > 0.55 {
                rgb(0x06, 0x12, 0x1f)
            } else {
                rgb(0xff, 0xff, 0xff)
            },
        };
        self
    }

    /// The triple for a status. `theme.severity(Severity::Danger)`.
    pub fn severity(&self, severity: Severity) -> StatusColors {
        match severity {
            Severity::Success => self.success,
            Severity::Warning => self.warning,
            Severity::Danger => self.danger,
            Severity::Info => self.info,
        }
    }
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    let (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) = (a, b) else {
        return a;
    };
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * amount).round() as u8;
    Color::Rgb(lerp(ar, br), lerp(ag, bg), lerp(ab, bb))
}

fn luminance(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}
