//! Initials avatar with a deterministic per-name hue and an optional status
//! dot. (Image sources come later with the `images` feature.)

use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use crate::widgets::Tone;
use egui::{Color32, Sense, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl AvatarSize {
    fn px(self) -> f32 {
        match self {
            AvatarSize::Sm => 24.0,
            AvatarSize::Md => 32.0,
            AvatarSize::Lg => 40.0,
        }
    }
}

pub struct Avatar<'a> {
    name: &'a str,
    size: AvatarSize,
    status: Option<Tone>,
}

impl<'a> Avatar<'a> {
    pub fn new(name: &'a str) -> Avatar<'a> {
        Avatar {
            name,
            size: AvatarSize::Md,
            status: None,
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// Show a status dot in the given tone at the bottom-right.
    pub fn status(mut self, tone: Tone) -> Self {
        self.status = Some(tone);
        self
    }

    fn initials(name: &str) -> String {
        let mut words = name.split_whitespace();
        let first = words.next().and_then(|w| w.chars().next());
        let second = words.last().and_then(|w| w.chars().next());
        match (first, second) {
            (Some(a), Some(b)) => format!("{}{}", a, b).to_uppercase(),
            (Some(a), None) => a.to_uppercase().to_string(),
            _ => "?".to_owned(),
        }
    }

    /// Deterministic per-name fill: FNV-1a hash → hue.
    fn fill(name: &str, t: &Theme) -> Color32 {
        let mut hash: u32 = 0x811c9dc5;
        for b in name.bytes() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(0x01000193);
        }
        let hue = (hash % 360) as f32 / 360.0;
        let (s, v) = match t.scheme {
            crate::theme::Scheme::Dark => (0.45, 0.45),
            crate::theme::Scheme::Light => (0.35, 0.75),
        };
        egui::ecolor::Hsva::new(hue, s, v, 1.0).into()
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let side = self.size.px();
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        if ui.is_rect_visible(rect) {
            let fill = Self::fill(self.name, &t);
            ui.painter().circle_filled(rect.center(), side / 2.0, fill);
            let font = t.font(ui.ctx(), FontWeight::Medium, (side * 0.38).round());
            let g = util::galley(ui, Self::initials(self.name), font, Color32::WHITE);
            ui.painter()
                .galley(rect.center() - g.size() / 2.0, g, Color32::WHITE);
            if let Some(tone) = self.status {
                let (base, _, _) = tone.triple(&t);
                let r = (side * 0.14).max(3.0);
                let center = rect.max - Vec2::splat(r);
                ui.painter()
                    .circle(center, r, base, Stroke::new(2.0, t.bg[1]));
            }
        }
        response.on_hover_text(self.name)
    }
}
