//! Tab bar with an accent underline on the active tab. Content is the
//! caller's `match` — this is just the bar (web parity).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use egui::{Sense, Stroke, Ui, Vec2};

pub struct TabItem<'a> {
    pub label: &'a str,
    pub count: Option<u32>,
    pub disabled: bool,
}

impl<'a> TabItem<'a> {
    pub fn new(label: &'a str) -> TabItem<'a> {
        TabItem {
            label,
            count: None,
            disabled: false,
        }
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct Tabs<'a> {
    active: &'a mut usize,
    items: &'a [TabItem<'a>],
}

impl<'a> Tabs<'a> {
    pub fn new(active: &'a mut usize, items: &'a [TabItem<'a>]) -> Tabs<'a> {
        Tabs { active, items }
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let mut outcome = Outcome::Ignored;
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (i, item) in self.items.iter().enumerate() {
                    let active = i == *self.active;
                    let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base);
                    let color = if item.disabled {
                        t.fg[3]
                    } else if active {
                        t.fg[0]
                    } else {
                        t.fg[2]
                    };
                    let mut text = item.label.to_owned();
                    if let Some(count) = item.count {
                        text.push_str(&format!("  {count}"));
                    }
                    let galley = ui.painter().layout_no_wrap(text, font, color);
                    let size = Vec2::new(galley.size().x + 24.0, 34.0);
                    let sense = if item.disabled {
                        Sense::hover()
                    } else {
                        Sense::click()
                    };
                    let (rect, r) = ui.allocate_exact_size(size, sense);
                    r.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::SelectableLabel,
                            !item.disabled,
                            active,
                            item.label,
                        )
                    });
                    if ui.is_rect_visible(rect) {
                        if r.hovered() && !item.disabled && !active {
                            ui.painter().rect_filled(
                                rect.shrink2(egui::vec2(2.0, 4.0)),
                                egui::CornerRadius::same(t.radius.sm as u8),
                                t.bg[2],
                            );
                        }
                        ui.painter().galley(
                            egui::pos2(rect.min.x + 12.0, rect.center().y - galley.size().y / 2.0),
                            galley,
                            color,
                        );
                        if active {
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.min.x + 8.0, rect.max.y - 1.0),
                                    egui::pos2(rect.max.x - 8.0, rect.max.y - 1.0),
                                ],
                                Stroke::new(2.0, t.accent.base),
                            );
                        }
                    }
                    if r.clicked() && !active {
                        *self.active = i;
                        outcome = Outcome::Changed;
                    }
                }
            })
            .response;

        // Baseline under the whole bar.
        let rect = response.rect;
        ui.painter().line_segment(
            [
                rect.left_bottom(),
                egui::pos2(ui.max_rect().max.x, rect.max.y),
            ],
            Stroke::new(1.0, t.border.subtle),
        );
        ForgeResponse::new(response, outcome)
    }
}
