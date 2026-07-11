//! Breadcrumb path. The last item is the current page (fg[0]); earlier items
//! are clickable.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::Theme;
use egui::Ui;

pub struct Crumbs<'a> {
    items: &'a [&'a str],
}

impl<'a> Crumbs<'a> {
    pub fn new(items: &'a [&'a str]) -> Crumbs<'a> {
        Crumbs { items }
    }

    /// `Changed` with the clicked ancestor's index available via
    /// [`Crumbs::show_indexed`]; plain `show` just reports the outcome.
    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        self.show_indexed(ui).0
    }

    pub fn show_indexed(self, ui: &mut Ui) -> (ForgeResponse, Option<usize>) {
        let t = Theme::of(ui.ctx());
        let mut clicked = None;
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let last = self.items.len().saturating_sub(1);
                for (i, item) in self.items.iter().enumerate() {
                    if i == last {
                        ui.label(egui::RichText::new(*item).color(t.fg[0]));
                    } else {
                        let r = ui.link(egui::RichText::new(*item).color(t.fg[2]));
                        if r.clicked() {
                            clicked = Some(i);
                        }
                        ui.label(egui::RichText::new("/").color(t.fg[3]));
                    }
                }
            })
            .response;
        let outcome = if clicked.is_some() {
            Outcome::Changed
        } else {
            Outcome::Ignored
        };
        (ForgeResponse::new(response, outcome), clicked)
    }
}
