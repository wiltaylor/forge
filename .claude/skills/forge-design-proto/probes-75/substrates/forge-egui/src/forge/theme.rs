use egui::Color32;

/// Forge colour roles, as written for this app.
pub struct Theme {
    pub bg_0: Color32,
    pub bg_1: Color32,
    pub bg_2: Color32,
    pub border: Color32,
    pub fg_0: Color32,
    pub fg_1: Color32,
    pub fg_2: Color32,
    pub accent: Color32,
    pub accent_contrast: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub danger: Color32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_0: Color32::from_rgb(0x0B, 0x0D, 0x10),
            bg_1: Color32::from_rgb(0x11, 0x14, 0x1A),
            bg_2: Color32::from_rgb(0x17, 0x1B, 0x22),
            border: Color32::from_rgb(0x26, 0x2C, 0x36),
            fg_0: Color32::from_rgb(0xE6, 0xEA, 0xF0),
            fg_1: Color32::from_rgb(0xA8, 0xB2, 0xC0),
            fg_2: Color32::from_rgb(0x6B, 0x76, 0x88),
            accent: Color32::from_rgb(0x4C, 0x8D, 0xFF),
            accent_contrast: Color32::WHITE,
            ok: Color32::from_rgb(0x3F, 0xB9, 0x50),
            warn: Color32::from_rgb(0xD2, 0x99, 0x22),
            danger: Color32::from_rgb(0xF8, 0x51, 0x49),
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = self.bg_0;
        visuals.window_fill = self.bg_1;
        ctx.set_visuals(visuals);
    }
}
