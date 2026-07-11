//! Radio group bound to a `usize` index. Each option is its own focusable
//! click target (Tab between options, Space/Enter selects).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use egui::{Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

const CIRCLE_R: f32 = 8.0;
const DOT_R: f32 = 4.0;
const GAP: f32 = 8.0;

pub struct RadioGroup<'a> {
    value: &'a mut usize,
    options: &'a [&'a str],
    row: bool,
    disabled: bool,
}

impl<'a> RadioGroup<'a> {
    pub fn new(value: &'a mut usize, options: &'a [&'a str]) -> RadioGroup<'a> {
        RadioGroup {
            value,
            options,
            row: false,
            disabled: false,
        }
    }

    /// Lay the options out horizontally.
    pub fn row(mut self, row: bool) -> Self {
        self.row = row;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self {
            value,
            options,
            row,
            disabled,
        } = self;

        let mut outcome = Outcome::Ignored;
        let mut union: Option<Response> = None;
        let mut draw = |ui: &mut Ui| {
            for (i, option) in options.iter().enumerate() {
                let response = option_ui(ui, &t, option, *value == i, disabled);
                if response.clicked() && !disabled && *value != i {
                    *value = i;
                    outcome = Outcome::Changed;
                }
                union = Some(match union.take() {
                    Some(u) => u.union(response),
                    None => response,
                });
            }
        };
        if row {
            ui.horizontal_wrapped(&mut draw);
        } else {
            ui.vertical(|ui| draw(ui));
        }

        let response = union.expect("RadioGroup needs at least one option");
        ForgeResponse::new(response, outcome)
    }
}

fn option_ui(ui: &mut Ui, t: &Theme, label: &str, selected: bool, disabled: bool) -> Response {
    let text_color = if disabled { t.fg[3] } else { t.fg[0] };
    let galley = util::galley(
        ui,
        label,
        t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
        text_color,
    );
    let side = CIRCLE_R * 2.0;
    let size = Vec2::new(
        side + GAP + galley.size().x,
        galley.size().y.max(side) + 4.0,
    );
    let sense = if disabled {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, !disabled, selected, label));

    if ui.is_rect_visible(rect) {
        let center = egui::pos2(rect.min.x + CIRCLE_R, rect.center().y);
        if disabled {
            ui.painter()
                .circle(center, CIRCLE_R, t.bg[2], Stroke::new(1.0, t.border.subtle));
            if selected {
                ui.painter().circle_filled(center, DOT_R, t.fg[3]);
            }
        } else if selected {
            ui.painter()
                .circle_stroke(center, CIRCLE_R - 0.75, Stroke::new(1.5, t.accent.base));
            ui.painter().circle_filled(center, DOT_R, t.accent.base);
        } else {
            let border = if response.hovered() {
                t.accent.base
            } else {
                t.border.strong
            };
            ui.painter()
                .circle(center, CIRCLE_R, t.bg[2], Stroke::new(1.0, border));
        }
        if response.has_focus() {
            ui.painter()
                .circle_stroke(center, CIRCLE_R + 2.0, Stroke::new(1.5, t.accent.base));
        }
        ui.painter().galley(
            egui::pos2(
                rect.min.x + side + GAP,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            text_color,
        );
    }
    response
}
