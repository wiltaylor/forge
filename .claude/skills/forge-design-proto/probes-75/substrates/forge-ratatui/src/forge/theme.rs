use ratatui::style::Color;

/// Forge colour roles, as written for this app.
pub struct Theme {
    pub bg_0: Color,
    pub bg_1: Color,
    pub bg_2: Color,
    pub border: Color,
    pub fg_0: Color,
    pub fg_1: Color,
    pub fg_2: Color,
    pub accent: Color,
    pub accent_contrast: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_0: Color::Rgb(0x0B, 0x0D, 0x10),
            bg_1: Color::Rgb(0x11, 0x14, 0x1A),
            bg_2: Color::Rgb(0x17, 0x1B, 0x22),
            border: Color::Rgb(0x26, 0x2C, 0x36),
            fg_0: Color::Rgb(0xE6, 0xEA, 0xF0),
            fg_1: Color::Rgb(0xA8, 0xB2, 0xC0),
            fg_2: Color::Rgb(0x6B, 0x76, 0x88),
            accent: Color::Rgb(0x4C, 0x8D, 0xFF),
            accent_contrast: Color::Rgb(0xFF, 0xFF, 0xFF),
            ok: Color::Rgb(0x3F, 0xB9, 0x50),
            warn: Color::Rgb(0xD2, 0x99, 0x22),
            danger: Color::Rgb(0xF8, 0x51, 0x49),
        }
    }
}
