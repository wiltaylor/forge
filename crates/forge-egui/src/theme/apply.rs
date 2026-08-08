//! Installing a [`Theme`] on an [`egui::Context`], and reading it back.
//!
//! Two layers: everything egui's `Style`/`Visuals` can express is mapped onto
//! them (so third-party egui widgets dropped into a Forge app look
//! approximately right), and the theme itself is stored in the context so
//! Forge widgets can read the full token set with [`Theme::of`] — the
//! tokens egui can't express (bg ramp levels, semantic triples, accent
//! hover/press split, control heights).

use super::{fonts, Scheme, Surface, TextRole, Theme};
use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Vec2};

fn theme_key() -> egui::Id {
    egui::Id::new("forge-egui-theme")
}

impl Theme {
    /// Install this theme on the context: fonts (once), egui style/visuals,
    /// text styles, and the theme itself for [`Theme::of`]. Call once at
    /// startup and again on theme switch — never per frame.
    pub fn apply(&self, ctx: &egui::Context) {
        fonts::install(ctx);

        let visuals = self.visuals();
        let theme = self.clone();
        ctx.all_styles_mut(move |style| {
            style.visuals = visuals.clone();
            style.animation_time = theme.motion.base;

            style.spacing.item_spacing = Vec2::new(8.0, 8.0);
            style.spacing.button_padding = Vec2::new(12.0, 6.0);
            style.spacing.interact_size = Vec2::new(40.0, theme.control.md);
            style.spacing.menu_margin = Margin::same(4);
            style.spacing.window_margin = Margin::same(16);

            use egui::{FontFamily, FontId, TextStyle};
            let ts = &theme.type_scale;
            style.text_styles = [
                (
                    TextStyle::Small,
                    FontId::new(ts.sm, FontFamily::Proportional),
                ),
                (
                    TextStyle::Body,
                    FontId::new(ts.base, FontFamily::Proportional),
                ),
                (
                    TextStyle::Button,
                    FontId::new(ts.base, FontFamily::Proportional),
                ),
                (
                    TextStyle::Monospace,
                    FontId::new(ts.base, FontFamily::Monospace),
                ),
                (
                    TextStyle::Heading,
                    FontId::new(ts.xl, fonts::family(super::FontWeight::SemiBold)),
                ),
            ]
            .into();
        });

        ctx.set_theme(match self.scheme {
            Scheme::Dark => egui::Theme::Dark,
            Scheme::Light => egui::Theme::Light,
        });

        ctx.data_mut(|d| d.insert_temp(theme_key(), self.clone()));
    }

    /// Read the installed theme back (dark fallback). This is how widgets get
    /// their tokens; it is a cheap clone of plain-data structs.
    pub fn of(ctx: &egui::Context) -> Theme {
        ctx.data(|d| d.get_temp::<Theme>(theme_key()))
            .unwrap_or_else(Theme::dark)
    }

    fn visuals(&self) -> egui::Visuals {
        let mut v = match self.scheme {
            Scheme::Dark => egui::Visuals::dark(),
            Scheme::Light => egui::Visuals::light(),
        };

        v.panel_fill = self.surface(Surface::Page);
        v.window_fill = self.surface(Surface::Popover);
        v.extreme_bg_color = self.surface(Surface::Card); // text-edit wells
        v.faint_bg_color = self.surface(Surface::Hover); // striped rows, subtle fills
        v.code_bg_color = self.surface(Surface::Hover);

        // Flat aesthetic: no shadows anywhere.
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        v.window_stroke = Stroke::new(1.0, self.border.default);
        v.window_corner_radius = CornerRadius::same(self.radius.lg as u8);
        v.menu_corner_radius = CornerRadius::same(self.radius.md as u8);

        v.selection.bg_fill = self.accent.bg;
        v.selection.stroke = Stroke::new(1.0, self.accent.base);
        v.hyperlink_color = self.accent.fg;
        v.warn_fg_color = self.warning.base;
        v.error_fg_color = self.danger.base;

        let radius = CornerRadius::same(self.radius.md as u8);
        let w = &mut v.widgets;

        // egui's state names sit one step below the Forge surface of the same
        // name: an `inactive` control rests on `Hover`, and `hovered`/`active`
        // both go to `Pressed`. The offset is deliberate — egui calls a resting
        // interactive control "inactive", which Forge paints as a raised
        // surface. Do not "correct" these to the matching role name.
        w.noninteractive.bg_fill = self.surface(Surface::Card);
        w.noninteractive.weak_bg_fill = self.surface(Surface::Card);
        w.noninteractive.bg_stroke = Stroke::new(1.0, self.border.subtle);
        w.noninteractive.fg_stroke = Stroke::new(1.0, self.text(TextRole::Secondary));
        w.noninteractive.corner_radius = radius;

        w.inactive.bg_fill = self.surface(Surface::Hover);
        w.inactive.weak_bg_fill = self.surface(Surface::Hover);
        w.inactive.bg_stroke = Stroke::new(1.0, self.border.default);
        w.inactive.fg_stroke = Stroke::new(1.0, self.text(TextRole::Primary));
        w.inactive.corner_radius = radius;

        w.hovered.bg_fill = self.surface(Surface::Pressed);
        w.hovered.weak_bg_fill = self.surface(Surface::Pressed);
        w.hovered.bg_stroke = Stroke::new(1.0, self.border.strong);
        w.hovered.fg_stroke = Stroke::new(1.0, self.text(TextRole::Primary));
        w.hovered.corner_radius = radius;
        w.hovered.expansion = 0.0;

        w.active.bg_fill = self.surface(Surface::Pressed);
        w.active.weak_bg_fill = self.surface(Surface::Pressed);
        w.active.bg_stroke = Stroke::new(1.0, self.accent.base);
        w.active.fg_stroke = Stroke::new(1.0, self.text(TextRole::Primary));
        w.active.corner_radius = radius;
        w.active.expansion = 0.0;

        w.open.bg_fill = self.surface(Surface::Pressed);
        w.open.weak_bg_fill = self.surface(Surface::Pressed);
        w.open.bg_stroke = Stroke::new(1.0, self.border.strong);
        w.open.fg_stroke = Stroke::new(1.0, self.text(TextRole::Primary));
        w.open.corner_radius = radius;

        v
    }
}

/// A translucent scrim color for overlays (modal backdrops, sheet scrims):
/// the page background at 60% alpha.
pub fn scrim(theme: &Theme) -> Color32 {
    super::color::with_alpha(theme.surface(Surface::Page), 153)
}
