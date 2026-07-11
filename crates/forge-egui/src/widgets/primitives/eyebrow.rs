//! Uppercase dim caption used above titles and sections.

use crate::theme::{FontWeight, Theme};
use egui::Ui;

pub struct Eyebrow<'a> {
    text: &'a str,
}

impl<'a> Eyebrow<'a> {
    pub fn new(text: &'a str) -> Eyebrow<'a> {
        Eyebrow { text }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        // Approximate the web's 0.08em tracking with hair spaces.
        let spaced: String = self
            .text
            .to_uppercase()
            .chars()
            .flat_map(|c| [c, '\u{200A}'])
            .collect();
        ui.label(
            egui::RichText::new(spaced.trim_end_matches('\u{200A}'))
                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs))
                .color(t.fg[2]),
        )
    }
}
