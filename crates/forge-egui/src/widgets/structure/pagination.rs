//! Page controls with ellipsis windows for long ranges (web parity: windows
//! with `…` beyond 7 pages).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::Theme;
use crate::widgets::{Button, Variant};
use egui::Ui;

pub struct Pagination<'a> {
    page: &'a mut usize,
    pages: usize,
}

impl<'a> Pagination<'a> {
    /// `page` is zero-based; `pages` is the total count.
    pub fn new(page: &'a mut usize, pages: usize) -> Pagination<'a> {
        Pagination { page, pages }
    }

    /// The visible page buttons: always first/last, a window around the
    /// current page, `None` = ellipsis.
    fn window(page: usize, pages: usize) -> Vec<Option<usize>> {
        if pages <= 7 {
            return (0..pages).map(Some).collect();
        }
        let mut items = vec![Some(0)];
        let lo = page.saturating_sub(1).max(1);
        let hi = (page + 1).min(pages - 2);
        if lo > 1 {
            items.push(None);
        }
        for i in lo..=hi {
            items.push(Some(i));
        }
        if hi < pages - 2 {
            items.push(None);
        }
        items.push(Some(pages - 1));
        items
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let mut outcome = Outcome::Ignored;
        let page = *self.page;
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if Button::new("‹")
                    .small(true)
                    .disabled(page == 0)
                    .show(ui)
                    .clicked()
                {
                    *self.page = page.saturating_sub(1);
                    outcome = Outcome::Changed;
                }
                for entry in Self::window(page, self.pages) {
                    match entry {
                        Some(i) => {
                            let label = (i + 1).to_string();
                            let variant = if i == page {
                                Variant::Primary
                            } else {
                                Variant::Ghost
                            };
                            if Button::new(&label)
                                .small(true)
                                .variant(variant)
                                .show(ui)
                                .clicked()
                                && i != page
                            {
                                *self.page = i;
                                outcome = Outcome::Changed;
                            }
                        }
                        None => {
                            ui.label(egui::RichText::new("…").color(t.fg[3]));
                        }
                    }
                }
                if Button::new("›")
                    .small(true)
                    .disabled(page + 1 >= self.pages)
                    .show(ui)
                    .clicked()
                {
                    *self.page = (page + 1).min(self.pages.saturating_sub(1));
                    outcome = Outcome::Changed;
                }
            })
            .response;
        ForgeResponse::new(response, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::Pagination;

    #[test]
    fn short_ranges_have_no_ellipsis() {
        let w = Pagination::window(2, 7);
        assert_eq!(w.len(), 7);
        assert!(w.iter().all(Option::is_some));
    }

    #[test]
    fn long_ranges_window_around_current() {
        let w = Pagination::window(5, 20);
        assert_eq!(w.first(), Some(&Some(0)));
        assert_eq!(w.last(), Some(&Some(19)));
        assert_eq!(w.iter().filter(|e| e.is_none()).count(), 2);
        assert!(w.contains(&Some(5)));
        assert!(w.contains(&Some(4)));
        assert!(w.contains(&Some(6)));
    }

    #[test]
    fn edges_only_get_one_ellipsis() {
        let w = Pagination::window(0, 20);
        assert_eq!(w.iter().filter(|e| e.is_none()).count(), 1);
        let w = Pagination::window(19, 20);
        assert_eq!(w.iter().filter(|e| e.is_none()).count(), 1);
    }
}
