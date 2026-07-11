//! Page title block: eyebrow, h1 title, sub line, and an actions row on the
//! right.

use crate::theme::{FontWeight, Theme};
use egui::Ui;

pub struct PageHead<'a> {
    title: &'a str,
    eyebrow: Option<&'a str>,
    sub: Option<&'a str>,
    actions: Option<SlotFn<'a>>,
}

type SlotFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl<'a> PageHead<'a> {
    pub fn new(title: &'a str) -> PageHead<'a> {
        PageHead {
            title,
            eyebrow: None,
            sub: None,
            actions: None,
        }
    }

    pub fn eyebrow(mut self, eyebrow: &'a str) -> Self {
        self.eyebrow = Some(eyebrow);
        self
    }

    // Named for parity with the web `sub` prop.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(mut self, sub: &'a str) -> Self {
        self.sub = Some(sub);
        self
    }

    pub fn actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let response = ui
            .horizontal(|ui| {
                ui.vertical(|ui| {
                    if let Some(eyebrow) = self.eyebrow {
                        crate::widgets::Eyebrow::new(eyebrow).show(ui);
                    }
                    ui.label(
                        egui::RichText::new(self.title)
                            .font(t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.h3))
                            .color(t.fg[0]),
                    );
                    if let Some(sub) = self.sub {
                        ui.label(
                            egui::RichText::new(sub)
                                .size(t.type_scale.sm)
                                .color(t.fg[2]),
                        );
                    }
                });
                if let Some(actions) = self.actions {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
                }
            })
            .response;
        ui.add_space(t.space.x(3.0));
        response
    }
}
