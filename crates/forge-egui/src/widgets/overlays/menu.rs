//! Menus: [`DropdownMenu`] (opens below a trigger) and [`context_menu`]
//! (opens at the pointer on right-click). Both share [`MenuItem`] rows and
//! return `Some(index)` when a row is chosen.

use crate::response::ForgeResponse;
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{
    CornerRadius, Frame, Key, Margin, Popup, Response, Sense, Stroke, Ui, Vec2, WidgetInfo,
    WidgetType,
};

/// One row of a dropdown/context menu.
#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: String,
    pub icon: Option<Glyph>,
    pub danger: bool,
    pub disabled: bool,
    pub separator_before: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> MenuItem {
        MenuItem {
            label: label.into(),
            icon: None,
            danger: false,
            disabled: false,
            separator_before: false,
        }
    }

    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Destructive action — painted in the danger tone.
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint a subtle separator line above this row.
    pub fn separator_before(mut self, separator_before: bool) -> Self {
        self.separator_before = separator_before;
        self
    }
}

fn menu_frame(t: &Theme) -> Frame {
    Frame::new()
        .fill(t.bg[4])
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(4))
}

fn separator(ui: &mut Ui, t: &Theme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
    ui.painter().line_segment(
        [
            egui::pos2(rect.min.x, rect.center().y),
            egui::pos2(rect.max.x, rect.center().y),
        ],
        Stroke::new(1.0, t.border.subtle),
    );
}

fn menu_row(ui: &mut Ui, t: &Theme, item: &MenuItem, highlighted: bool) -> Response {
    let sense = if item.disabled {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), t.control.sm), sense);
    let label = item.label.clone();
    let enabled = !item.disabled;
    response.widget_info(move || WidgetInfo::labeled(WidgetType::Button, enabled, &label));

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered() && !item.disabled;
        if hovered || (highlighted && !item.disabled) {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(t.radius.sm as u8), t.bg[2]);
        }
        let color = if item.disabled {
            t.fg[3]
        } else if item.danger {
            t.danger.base
        } else if hovered || highlighted {
            t.fg[0]
        } else {
            t.fg[1]
        };
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base);
        let mut x = rect.min.x + 8.0;
        if let Some(icon) = item.icon {
            let g = util::galley(ui, icon.as_str(), font.clone(), color);
            ui.painter()
                .galley(egui::pos2(x, rect.center().y - g.size().y / 2.0), g, color);
            x += 18.0;
        }
        let g = util::galley(ui, &item.label, font, color);
        ui.painter()
            .galley(egui::pos2(x, rect.center().y - g.size().y / 2.0), g, color);
    }
    response
}

fn highlight(ctx: &egui::Context, id: egui::Id) -> usize {
    ctx.data(|d| d.get_temp(id)).unwrap_or(0)
}

fn set_highlight(ctx: &egui::Context, id: egui::Id, value: usize) {
    ctx.data_mut(|d| d.insert_temp(id, value));
}

/// A menu toggled by a trigger widget. Open state lives in egui's popup
/// memory; the keyboard highlight in temp memory keyed by the trigger id.
pub struct DropdownMenu<'a> {
    items: &'a [MenuItem],
    min_width: f32,
}

impl<'a> DropdownMenu<'a> {
    pub fn new(items: &'a [MenuItem]) -> DropdownMenu<'a> {
        DropdownMenu {
            items,
            min_width: 180.0,
        }
    }

    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Show the trigger and, while open, the menu below it. Returns
    /// `Some(index)` on selection (click or ↑/↓ + Enter).
    pub fn show(
        self,
        ui: &mut Ui,
        trigger: impl FnOnce(&mut Ui) -> ForgeResponse,
    ) -> Option<usize> {
        let t = Theme::of(ui.ctx());
        let response = trigger(ui);
        let popup_id = Popup::default_response_id(&response.response);
        let hl_id = response.id.with("forge-menu-hl");
        let was_open = Popup::is_id_open(ui.ctx(), popup_id);

        let enabled: Vec<usize> = (0..self.items.len())
            .filter(|&i| !self.items[i].disabled)
            .collect();

        let mut selected = None;
        if was_open {
            let (up, down, enter) = ui.input(|i| {
                (
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::Enter),
                )
            });
            if !enabled.is_empty() {
                let current = highlight(ui.ctx(), hl_id);
                let mut pos = enabled
                    .iter()
                    .position(|&i| i == current)
                    .unwrap_or_default();
                if down {
                    pos = (pos + 1).min(enabled.len() - 1);
                }
                if up {
                    pos = pos.saturating_sub(1);
                }
                set_highlight(ui.ctx(), hl_id, enabled[pos]);
                if enter {
                    selected = Some(enabled[pos]);
                    Popup::close_id(ui.ctx(), popup_id);
                }
            }
            if response.has_focus() {
                // Keep arrows moving the highlight, not egui focus.
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        response.id,
                        egui::EventFilter {
                            vertical_arrows: true,
                            ..Default::default()
                        },
                    );
                });
            }
        } else if response.clicked() {
            set_highlight(ui.ctx(), hl_id, enabled.first().copied().unwrap_or(0));
        }

        let hl = highlight(ui.ctx(), hl_id);
        Popup::from_toggle_button_response(&response.response)
            .gap(4.0)
            .frame(menu_frame(&t))
            .show(|ui| {
                ui.set_min_width(self.min_width);
                for (i, item) in self.items.iter().enumerate() {
                    if item.separator_before {
                        separator(ui, &t);
                    }
                    let row = menu_row(ui, &t, item, i == hl && was_open);
                    if row.clicked() {
                        selected = Some(i);
                    }
                }
            });

        selected
    }
}

/// A right-click menu on `response`'s widget, opened at the pointer.
/// Returns `Some(index)` when a row is chosen; Esc/click-away closes.
pub fn context_menu(response: &Response, items: &[MenuItem]) -> Option<usize> {
    let t = Theme::of(&response.ctx);
    let mut selected = None;
    Popup::context_menu(response)
        .frame(menu_frame(&t))
        .show(|ui| {
            ui.set_min_width(180.0);
            for (i, item) in items.iter().enumerate() {
                if item.separator_before {
                    separator(ui, &t);
                }
                if menu_row(ui, &t, item, false).clicked() {
                    selected = Some(i);
                }
            }
        });
    selected
}
