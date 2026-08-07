//! The Forge token set for egui.
//!
//! Field names and geometry come from `reference/tokens.md`. `Theme::apply` installs the
//! theme on the `egui::Context` once, at startup; a widget then reads it back with
//! `Theme::from_ctx` rather than taking it as an argument (`reference/egui.md`).
//!
//! The concrete colour values are not in the skill — only the roles are. See the note at
//! the head of `dark()`.

use egui::{Color32, CornerRadius, FontFamily, FontId, Id, Stroke, TextStyle};

/// Sidebar width, `reference/tokens.md`.
pub const SIDEBAR_WIDTH: f32 = 240.0;
/// Collapsed sidebar rail width.
pub const SIDEBAR_RAIL: f32 = 56.0;
/// Top bar height.
pub const TOPBAR_HEIGHT: f32 = 48.0;
/// Status bar height.
pub const STATUSBAR_HEIGHT: f32 = 28.0;

const THEME_KEY: &str = "forge/theme";

/// Dark is the default. Forge never ships light-only (`reference/laws.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

/// There is no bold (`reference/tokens.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Regular,
    Medium,
    SemiBold,
}

impl FontWeight {
    /// The family name an app registers if it ships weighted faces. When the family is not
    /// installed, `Theme::font` falls back to the proportional family.
    pub fn family_name(self) -> &'static str {
        match self {
            FontWeight::Regular => "forge-regular",
            FontWeight::Medium => "forge-medium",
            FontWeight::SemiBold => "forge-semibold",
        }
    }
}

/// The four statuses. Status colour never carries meaning alone (`reference/laws.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Success,
    Warning,
    Danger,
    Info,
}

/// `base` is the solid, `bg` a 14% tint, `fg` the text that goes on that tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusTokens {
    pub base: Color32,
    pub bg: Color32,
    pub fg: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderTokens {
    pub default: Color32,
    pub subtle: Color32,
    pub strong: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccentTokens {
    pub base: Color32,
    pub hover: Color32,
    pub press: Color32,
    /// 14% tint.
    pub bg: Color32,
    /// Accent as text on a surface.
    pub fg: Color32,
    /// Text on a solid accent.
    pub contrast: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadiusTokens {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

/// `theme.space.x(n)` = n × 4.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceScale {
    pub step: f32,
}

impl SpaceScale {
    pub fn x(self, n: i32) -> f32 {
        n as f32 * self.step
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlHeights {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

/// Seconds, not milliseconds — egui counts time in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionTokens {
    pub fast: f32,
    pub base: f32,
    pub slow: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg: [Color32; 5],
    pub fg: [Color32; 4],
    pub border: BorderTokens,
    pub accent: AccentTokens,
    pub success: StatusTokens,
    pub warning: StatusTokens,
    pub danger: StatusTokens,
    pub info: StatusTokens,
    pub radius: RadiusTokens,
    pub space: SpaceScale,
    pub control: ControlHeights,
    pub type_scale: TypeScale,
    pub motion: MotionTokens,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::dark()
    }
}

impl Theme {
    /// The default theme.
    ///
    /// `reference/tokens.md` names every field but gives no hex values, so the ramp below
    /// is chosen here: a neutral cool-grey surface stack and a blue accent, dense and flat.
    /// Replace the literals to rebrand; no other file holds a colour.
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg: [
                Color32::from_rgb(0x0d, 0x0f, 0x12),
                Color32::from_rgb(0x14, 0x17, 0x1c),
                Color32::from_rgb(0x1b, 0x1f, 0x26),
                Color32::from_rgb(0x23, 0x28, 0x30),
                Color32::from_rgb(0x2a, 0x30, 0x3a),
            ],
            fg: [
                Color32::from_rgb(0xe6, 0xe9, 0xef),
                Color32::from_rgb(0xb8, 0xbf, 0xcc),
                Color32::from_rgb(0x8b, 0x93, 0xa3),
                Color32::from_rgb(0x5a, 0x61, 0x6e),
            ],
            border: BorderTokens {
                default: Color32::from_rgb(0x26, 0x2b, 0x33),
                subtle: Color32::from_rgb(0x1c, 0x20, 0x27),
                strong: Color32::from_rgb(0x39, 0x40, 0x4b),
            },
            accent: AccentTokens {
                base: Color32::from_rgb(0x4c, 0x8d, 0xff),
                hover: Color32::from_rgb(0x6b, 0xa0, 0xff),
                press: Color32::from_rgb(0x3a, 0x7a, 0xe0),
                bg: tint(Color32::from_rgb(0x4c, 0x8d, 0xff)),
                fg: Color32::from_rgb(0x8f, 0xb6, 0xff),
                contrast: Color32::from_rgb(0x0b, 0x10, 0x20),
            },
            success: StatusTokens {
                base: Color32::from_rgb(0x30, 0xa4, 0x6c),
                bg: tint(Color32::from_rgb(0x30, 0xa4, 0x6c)),
                fg: Color32::from_rgb(0x6d, 0xd4, 0xa4),
            },
            warning: StatusTokens {
                base: Color32::from_rgb(0xd9, 0x8a, 0x1a),
                bg: tint(Color32::from_rgb(0xd9, 0x8a, 0x1a)),
                fg: Color32::from_rgb(0xf5, 0xbf, 0x6b),
            },
            danger: StatusTokens {
                base: Color32::from_rgb(0xe0, 0x45, 0x4a),
                bg: tint(Color32::from_rgb(0xe0, 0x45, 0x4a)),
                fg: Color32::from_rgb(0xff, 0x9b, 0x9e),
            },
            info: StatusTokens {
                base: Color32::from_rgb(0x53, 0x94, 0xd1),
                bg: tint(Color32::from_rgb(0x53, 0x94, 0xd1)),
                fg: Color32::from_rgb(0x9a, 0xc6, 0xed),
            },
            ..Theme::geometry(ThemeMode::Dark)
        }
    }

    /// The light theme. Same roles, same geometry, different ramp.
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg: [
                Color32::from_rgb(0xff, 0xff, 0xff),
                Color32::from_rgb(0xf7, 0xf8, 0xfa),
                Color32::from_rgb(0xee, 0xf0, 0xf4),
                Color32::from_rgb(0xe4, 0xe7, 0xec),
                Color32::from_rgb(0xff, 0xff, 0xff),
            ],
            fg: [
                Color32::from_rgb(0x14, 0x17, 0x1c),
                Color32::from_rgb(0x3d, 0x44, 0x4f),
                Color32::from_rgb(0x65, 0x6d, 0x7a),
                Color32::from_rgb(0x99, 0xa0, 0xab),
            ],
            border: BorderTokens {
                default: Color32::from_rgb(0xd8, 0xdc, 0xe2),
                subtle: Color32::from_rgb(0xe8, 0xea, 0xee),
                strong: Color32::from_rgb(0xb9, 0xc0, 0xc9),
            },
            accent: AccentTokens {
                base: Color32::from_rgb(0x25, 0x63, 0xeb),
                hover: Color32::from_rgb(0x1d, 0x4e, 0xd8),
                press: Color32::from_rgb(0x1e, 0x40, 0xaf),
                bg: tint(Color32::from_rgb(0x25, 0x63, 0xeb)),
                fg: Color32::from_rgb(0x1d, 0x4e, 0xd8),
                contrast: Color32::from_rgb(0xff, 0xff, 0xff),
            },
            success: StatusTokens {
                base: Color32::from_rgb(0x18, 0x79, 0x4e),
                bg: tint(Color32::from_rgb(0x18, 0x79, 0x4e)),
                fg: Color32::from_rgb(0x0f, 0x5c, 0x3a),
            },
            warning: StatusTokens {
                base: Color32::from_rgb(0xa8, 0x5c, 0x00),
                bg: tint(Color32::from_rgb(0xa8, 0x5c, 0x00)),
                fg: Color32::from_rgb(0x7c, 0x44, 0x00),
            },
            danger: StatusTokens {
                base: Color32::from_rgb(0xc4, 0x25, 0x2c),
                bg: tint(Color32::from_rgb(0xc4, 0x25, 0x2c)),
                fg: Color32::from_rgb(0x9b, 0x1c, 0x22),
            },
            info: StatusTokens {
                base: Color32::from_rgb(0x1f, 0x6f, 0xb2),
                bg: tint(Color32::from_rgb(0x1f, 0x6f, 0xb2)),
                fg: Color32::from_rgb(0x18, 0x55, 0x87),
            },
            ..Theme::geometry(ThemeMode::Light)
        }
    }

    /// Derive hover, press, tint and text from one brand colour.
    pub fn with_accent(mut self, base: Color32) -> Self {
        let dark = self.mode == ThemeMode::Dark;
        self.accent = AccentTokens {
            base,
            hover: if dark {
                lighten(base, 0.14)
            } else {
                darken(base, 0.12)
            },
            press: darken(base, 0.16),
            bg: tint(base),
            fg: if dark {
                lighten(base, 0.28)
            } else {
                darken(base, 0.12)
            },
            contrast: contrast_on(base),
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

    /// A text font. Weight resolves to a registered family when the app installs one, and
    /// falls back to egui's proportional family when it does not.
    pub fn font(&self, ctx: &egui::Context, weight: FontWeight, size: f32) -> FontId {
        let family = FontFamily::Name(weight.family_name().into());
        let installed = ctx.fonts(|f| f.families().contains(&family));
        if installed {
            FontId::new(size, family)
        } else {
            FontId::new(size, FontFamily::Proportional)
        }
    }

    /// The mono font, for numbers, ids and log bodies.
    pub fn mono(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Monospace)
    }

    /// A token radius as an egui corner radius.
    pub fn corner(&self, radius: f32) -> CornerRadius {
        CornerRadius::same(radius.round().clamp(0.0, 255.0) as u8)
    }

    /// Install the theme on the context. Call once, at startup, and again on a theme
    /// switch. After this a widget reads the theme back with `Theme::from_ctx`.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style_of(ctx.theme())).clone();
        let mut visuals = match self.mode {
            ThemeMode::Dark => egui::Visuals::dark(),
            ThemeMode::Light => egui::Visuals::light(),
        };

        visuals.dark_mode = self.mode == ThemeMode::Dark;
        visuals.override_text_color = Some(self.fg[0]);
        visuals.panel_fill = self.bg[0];
        visuals.window_fill = self.bg[1];
        visuals.faint_bg_color = self.bg[1];
        visuals.extreme_bg_color = self.bg[0];
        visuals.window_stroke = Stroke::new(1.0, self.border.default);
        visuals.window_corner_radius = self.corner(self.radius.md);
        visuals.menu_corner_radius = self.corner(self.radius.md);
        visuals.selection.bg_fill = self.accent.bg;
        visuals.selection.stroke = Stroke::new(1.0, self.accent.base);
        visuals.hyperlink_color = self.accent.fg;
        // Forge separates layers with a surface step and a border, never with a shadow.
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.popup_shadow = egui::epaint::Shadow::NONE;

        let widgets = &mut visuals.widgets;
        for (w, fill) in [
            (&mut widgets.noninteractive, self.bg[1]),
            (&mut widgets.inactive, self.bg[1]),
            (&mut widgets.hovered, self.bg[2]),
            (&mut widgets.active, self.bg[3]),
            (&mut widgets.open, self.bg[2]),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::new(1.0, self.border.default);
            w.fg_stroke = Stroke::new(1.0, self.fg[0]);
            w.corner_radius = self.corner(self.radius.sm);
            w.expansion = 0.0;
        }
        widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border.subtle);
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.fg[1]);
        style.visuals = visuals;

        style.spacing.item_spacing = egui::vec2(self.space.x(2), self.space.x(2));
        style.spacing.button_padding = egui::vec2(self.space.x(3), self.space.x(2));
        style.spacing.interact_size = egui::vec2(0.0, self.control.md);
        style.spacing.menu_margin = egui::Margin::same(self.space.x(1) as i8);
        style.spacing.window_margin = egui::Margin::same(self.space.x(4) as i8);

        style.text_styles = [
            (
                TextStyle::Small,
                self.font(ctx, FontWeight::Regular, self.type_scale.xs),
            ),
            (
                TextStyle::Body,
                self.font(ctx, FontWeight::Regular, self.type_scale.base),
            ),
            (
                TextStyle::Button,
                self.font(ctx, FontWeight::Medium, self.type_scale.base),
            ),
            (
                TextStyle::Heading,
                self.font(ctx, FontWeight::SemiBold, self.type_scale.h3),
            ),
            (TextStyle::Monospace, self.mono(self.type_scale.sm)),
        ]
        .into();

        // Both egui themes carry the Forge style, so an OS theme switch never undoes it.
        ctx.all_styles_mut(|s| *s = style.clone());
        ctx.data_mut(|d| d.insert_temp(Id::new(THEME_KEY), *self));
    }

    /// Read the theme a widget should paint with. Falls back to the default theme when no
    /// `apply` has run, so a widget never panics in a bare eframe app.
    pub fn from_ctx(ctx: &egui::Context) -> Theme {
        ctx.data(|d| d.get_temp::<Theme>(Id::new(THEME_KEY)))
            .unwrap_or_default()
    }

    fn geometry(mode: ThemeMode) -> Theme {
        let base = Color32::from_rgb(0x4c, 0x8d, 0xff);
        Theme {
            mode,
            bg: [Color32::BLACK; 5],
            fg: [Color32::WHITE; 4],
            border: BorderTokens {
                default: Color32::GRAY,
                subtle: Color32::GRAY,
                strong: Color32::GRAY,
            },
            accent: AccentTokens {
                base,
                hover: base,
                press: base,
                bg: tint(base),
                fg: base,
                contrast: Color32::BLACK,
            },
            success: StatusTokens {
                base,
                bg: base,
                fg: base,
            },
            warning: StatusTokens {
                base,
                bg: base,
                fg: base,
            },
            danger: StatusTokens {
                base,
                bg: base,
                fg: base,
            },
            info: StatusTokens {
                base,
                bg: base,
                fg: base,
            },
            radius: RadiusTokens {
                sm: 4.0,
                md: 6.0,
                lg: 8.0,
            },
            space: SpaceScale { step: 4.0 },
            control: ControlHeights {
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
}

/// The 14% tint of a solid.
fn tint(c: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 36)
}

fn lighten(c: Color32, amount: f32) -> Color32 {
    mix(c, Color32::WHITE, amount)
}

fn darken(c: Color32, amount: f32) -> Color32 {
    mix(c, Color32::BLACK, amount)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

/// Black or white, whichever reads better on the given solid.
fn contrast_on(c: Color32) -> Color32 {
    let luma = 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
    if luma > 140.0 {
        Color32::from_rgb(0x0b, 0x10, 0x20)
    } else {
        Color32::WHITE
    }
}
