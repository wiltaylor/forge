//! Forge theme for egui.
//!
//! `reference/tokens.md` names every field below. Nothing outside this file may
//! write a colour, a radius, a size or a duration as a literal.
//!
//! `Theme::apply` installs the theme on the `egui::Context` once, at startup.
//! A widget then reads it back with `Theme::get`.

use egui::{Color32, Context, CornerRadius, FontFamily, FontId, Id, Margin, Stroke};

/// Sidebar width — `SIDEBAR_WIDTH` in `tokens.md`.
pub const SIDEBAR_WIDTH: f32 = 240.0;
/// Collapsed sidebar rail — `SIDEBAR_RAIL` in `tokens.md`.
pub const SIDEBAR_RAIL: f32 = 56.0;
/// Top bar height — `TOPBAR_HEIGHT` in `tokens.md`.
pub const TOPBAR_HEIGHT: f32 = 48.0;
/// Status bar height — `STATUSBAR_HEIGHT` in `tokens.md`.
pub const STATUSBAR_HEIGHT: f32 = 28.0;

/// The three weights Forge ships. There is no bold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Medium,
    SemiBold,
}

impl FontWeight {
    /// The family a Forge build registers for this weight, if it has one.
    fn family(self) -> FontFamily {
        match self {
            Self::Regular => FontFamily::Proportional,
            Self::Medium => FontFamily::Name("forge-medium".into()),
            Self::SemiBold => FontFamily::Name("forge-semibold".into()),
        }
    }
}

/// The four statuses. `tokens.md` spells them `success` and `warning`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Success,
    Warning,
    Danger,
    Info,
}

/// A status triple: the solid, the 14% tint, and the text that goes on the tint.
#[derive(Clone, Copy, Debug)]
pub struct StatusTokens {
    pub base: Color32,
    pub bg: Color32,
    pub fg: Color32,
}

/// The accent role. One per screen.
#[derive(Clone, Copy, Debug)]
pub struct AccentTokens {
    pub base: Color32,
    pub hover: Color32,
    pub press: Color32,
    /// The 14% tint.
    pub bg: Color32,
    /// Accent as text.
    pub fg: Color32,
    /// Text on a solid accent fill.
    pub contrast: Color32,
}

/// Every 1px division.
#[derive(Clone, Copy, Debug)]
pub struct BorderTokens {
    pub default: Color32,
    pub subtle: Color32,
    pub strong: Color32,
}

#[derive(Clone, Copy, Debug)]
pub struct RadiusTokens {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

/// The spacing step. `x(n)` is n × 4.0.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpaceTokens;

impl SpaceTokens {
    pub fn x(self, n: f32) -> f32 {
        n * 4.0
    }
}

/// Control heights.
#[derive(Clone, Copy, Debug)]
pub struct ControlTokens {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TypeScale {
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub md: f32,
    pub lg: f32,
    pub h3: f32,
    pub h2: f32,
    pub h1: f32,
}

/// Durations, in seconds.
#[derive(Clone, Copy, Debug)]
pub struct MotionTokens {
    pub fast: f32,
    pub base: f32,
    pub slow: f32,
}

/// The Forge theme. Dark is the default; light is never optional.
#[derive(Clone, Debug)]
pub struct Theme {
    pub dark: bool,
    /// `bg[0]` page · `bg[1]` raised · `bg[2]` hover · `bg[3]` active · `bg[4]` popover.
    pub bg: [Color32; 5],
    /// `fg[0]` text · `fg[1]` secondary · `fg[2]` dim · `fg[3]` disabled and placeholder.
    pub fg: [Color32; 4],
    pub border: BorderTokens,
    pub accent: AccentTokens,
    pub success: StatusTokens,
    pub warning: StatusTokens,
    pub danger: StatusTokens,
    pub info: StatusTokens,
    pub radius: RadiusTokens,
    pub space: SpaceTokens,
    pub control: ControlTokens,
    pub type_scale: TypeScale,
    pub motion: MotionTokens,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: [
                rgb(0x0d1117),
                rgb(0x131a22),
                rgb(0x1b2430),
                rgb(0x243040),
                rgb(0x161e27),
            ],
            fg: [rgb(0xe6edf3), rgb(0xb6c2cf), rgb(0x8b98a5), rgb(0x5c6773)],
            border: BorderTokens {
                default: rgb(0x263140),
                subtle: rgb(0x1c242e),
                strong: rgb(0x38455a),
            },
            accent: AccentTokens {
                base: rgb(0x4c8dff),
                hover: rgb(0x6ba1ff),
                press: rgb(0x3573e6),
                bg: rgb(0x18263c),
                fg: rgb(0x8fbaff),
                contrast: rgb(0x08121f),
            },
            success: StatusTokens {
                base: rgb(0x3fb950),
                bg: rgb(0x122119),
                fg: rgb(0x7ee08b),
            },
            warning: StatusTokens {
                base: rgb(0xd29922),
                bg: rgb(0x231d10),
                fg: rgb(0xe3b341),
            },
            danger: StatusTokens {
                base: rgb(0xf85149),
                bg: rgb(0x2a1517),
                fg: rgb(0xff8a83),
            },
            info: StatusTokens {
                base: rgb(0x58a6ff),
                bg: rgb(0x122033),
                fg: rgb(0x8fbaff),
            },
            ..Self::geometry()
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg: [
                rgb(0xffffff),
                rgb(0xf6f8fa),
                rgb(0xeef1f5),
                rgb(0xe3e8ee),
                rgb(0xffffff),
            ],
            fg: [rgb(0x11181f), rgb(0x33404d), rgb(0x5b6875), rgb(0x8b98a5)],
            border: BorderTokens {
                default: rgb(0xd5dbe2),
                subtle: rgb(0xe6eaef),
                strong: rgb(0xb3bcc7),
            },
            accent: AccentTokens {
                base: rgb(0x1f6feb),
                hover: rgb(0x3b82f6),
                press: rgb(0x1a5fcc),
                bg: rgb(0xe4edfd),
                fg: rgb(0x1a5fcc),
                contrast: rgb(0xffffff),
            },
            success: StatusTokens {
                base: rgb(0x1a7f37),
                bg: rgb(0xe6f4ea),
                fg: rgb(0x11602a),
            },
            warning: StatusTokens {
                base: rgb(0x9a6700),
                bg: rgb(0xfaf2dd),
                fg: rgb(0x7a5200),
            },
            danger: StatusTokens {
                base: rgb(0xcf222e),
                bg: rgb(0xfbe9ea),
                fg: rgb(0xa40e1c),
            },
            info: StatusTokens {
                base: rgb(0x0969da),
                bg: rgb(0xe4edfd),
                fg: rgb(0x0757ba),
            },
            ..Self::geometry()
        }
    }

    /// Everything that does not change with the colour mode.
    fn geometry() -> Self {
        Self {
            dark: true,
            bg: [Color32::TRANSPARENT; 5],
            fg: [Color32::TRANSPARENT; 4],
            border: BorderTokens {
                default: Color32::TRANSPARENT,
                subtle: Color32::TRANSPARENT,
                strong: Color32::TRANSPARENT,
            },
            accent: AccentTokens {
                base: Color32::TRANSPARENT,
                hover: Color32::TRANSPARENT,
                press: Color32::TRANSPARENT,
                bg: Color32::TRANSPARENT,
                fg: Color32::TRANSPARENT,
                contrast: Color32::TRANSPARENT,
            },
            success: StatusTokens {
                base: Color32::TRANSPARENT,
                bg: Color32::TRANSPARENT,
                fg: Color32::TRANSPARENT,
            },
            warning: StatusTokens {
                base: Color32::TRANSPARENT,
                bg: Color32::TRANSPARENT,
                fg: Color32::TRANSPARENT,
            },
            danger: StatusTokens {
                base: Color32::TRANSPARENT,
                bg: Color32::TRANSPARENT,
                fg: Color32::TRANSPARENT,
            },
            info: StatusTokens {
                base: Color32::TRANSPARENT,
                bg: Color32::TRANSPARENT,
                fg: Color32::TRANSPARENT,
            },
            radius: RadiusTokens {
                sm: 4.0,
                md: 6.0,
                lg: 8.0,
            },
            space: SpaceTokens,
            control: ControlTokens {
                sm: 28.0,
                md: 32.0,
                lg: 36.0,
                xl: 40.0,
            },
            type_scale: TypeScale {
                xs: 11.0,
                sm: 12.0,
                base: 14.0,
                md: 16.0,
                lg: 18.0,
                h3: 22.0,
                h2: 28.0,
                h1: 34.0,
            },
            motion: MotionTokens {
                fast: 0.08,
                base: 0.16,
                slow: 0.24,
            },
        }
    }

    /// Derive hover, press, tint and accent text from one brand colour.
    pub fn with_accent(mut self, base: Color32) -> Self {
        let toward = |c: Color32, other: Color32, t: f32| {
            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
            Color32::from_rgb(
                mix(c.r(), other.r()),
                mix(c.g(), other.g()),
                mix(c.b(), other.b()),
            )
        };
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        let (lift, sink) = if self.dark { (white, black) } else { (black, white) };
        self.accent = AccentTokens {
            base,
            hover: toward(base, lift, 0.18),
            press: toward(base, sink, 0.18),
            bg: toward(self.bg[0], base, 0.14),
            fg: if self.dark {
                toward(base, white, 0.35)
            } else {
                toward(base, black, 0.15)
            },
            contrast: if self.dark { self.bg[0] } else { white },
        };
        self
    }

    /// The triple for a status.
    pub fn severity(&self, severity: Severity) -> StatusTokens {
        match severity {
            Severity::Success => self.success,
            Severity::Warning => self.warning,
            Severity::Danger => self.danger,
            Severity::Info => self.info,
        }
    }

    /// A sans font at a weight and a size. Weights fall back to the regular
    /// family when the build has not registered a weighted family.
    pub fn font(&self, ctx: &Context, weight: FontWeight, size: f32) -> FontId {
        let family = weight.family();
        let known = ctx.fonts(|f| f.families().contains(&family));
        FontId::new(
            size,
            if known { family } else { FontFamily::Proportional },
        )
    }

    /// The mono font at a size. Numbers are tabular; they belong here.
    pub fn mono(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }

    fn slot() -> Id {
        Id::new("forge-theme")
    }

    /// Install the theme on the context. Call once at startup, and again only
    /// when the colour mode changes.
    pub fn apply(self, ctx: &Context) {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        let cr = CornerRadius::same(self.radius.sm as u8);
        visuals.panel_fill = self.bg[0];
        visuals.window_fill = self.bg[4];
        visuals.extreme_bg_color = self.bg[0];
        visuals.faint_bg_color = self.bg[1];
        visuals.code_bg_color = self.bg[0];
        visuals.window_stroke = Stroke::new(1.0, self.border.default);
        visuals.window_corner_radius = CornerRadius::same(self.radius.md as u8);
        visuals.menu_corner_radius = CornerRadius::same(self.radius.md as u8);
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.popup_shadow = egui::epaint::Shadow::NONE;
        visuals.override_text_color = Some(self.fg[0]);
        visuals.hyperlink_color = self.accent.fg;
        visuals.selection.bg_fill = self.accent.bg;
        visuals.selection.stroke = Stroke::new(1.0, self.accent.base);
        visuals.text_cursor.stroke = Stroke::new(1.0, self.accent.base);

        for (widget, fill) in [
            (&mut visuals.widgets.noninteractive, self.bg[1]),
            (&mut visuals.widgets.inactive, self.bg[1]),
            (&mut visuals.widgets.hovered, self.bg[2]),
            (&mut visuals.widgets.active, self.bg[3]),
            (&mut visuals.widgets.open, self.bg[2]),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.bg_stroke = Stroke::new(1.0, self.border.default);
            widget.fg_stroke = Stroke::new(1.0, self.fg[0]);
            widget.corner_radius = cr;
            widget.expansion = 0.0;
        }
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.fg[1]);

        ctx.set_visuals(visuals);

        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(self.space.x(2.0), self.space.x(2.0));
            style.spacing.button_padding = egui::vec2(self.space.x(3.0), self.space.x(1.0));
            style.spacing.interact_size.y = self.control.md;
            style.spacing.window_margin = Margin::same(self.space.x(2.0) as i8);
            style.spacing.menu_margin = Margin::same(self.space.x(1.0) as i8);
            style.spacing.scroll.bar_width = self.space.x(2.0);
            style.text_styles.insert(
                egui::TextStyle::Body,
                FontId::new(self.type_scale.base, FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                FontId::new(self.type_scale.base, FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Small,
                FontId::new(self.type_scale.sm, FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                FontId::new(self.type_scale.h2, FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Monospace,
                FontId::new(self.type_scale.sm, FontFamily::Monospace),
            );
        });

        ctx.data_mut(|d| d.insert_temp(Self::slot(), self));
    }

    /// Read the installed theme back. Falls back to dark when nothing is
    /// installed, so a widget never paints with no tokens at all.
    pub fn get(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp::<Self>(Self::slot()))
            .unwrap_or_else(Self::dark)
    }
}
