//! Input-style field + Calendar flyout on `egui::Popup` — the same pattern
//! as `forms::select`, with Input's label/help/error chrome.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::date::calendar::{Calendar, CalendarState};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{
    Color32, CornerRadius, Key, Margin, Popup, PopupCloseBehavior, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo, WidgetType,
};

/// Open flag + the embedded calendar. `state.cal.value` holds the pick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DatePickerState {
    pub open: bool,
    pub cal: CalendarState,
}

pub struct DatePicker<'a> {
    state: &'a mut DatePickerState,
    label: Option<&'a str>,
    help: Option<&'a str>,
    error: Option<&'a str>,
    placeholder: Option<&'a str>,
    min: Option<&'a str>,
    max: Option<&'a str>,
    disabled: bool,
    width: f32,
}

impl<'a> DatePicker<'a> {
    pub fn new(state: &'a mut DatePickerState) -> DatePicker<'a> {
        DatePicker {
            state,
            label: None,
            help: None,
            error: None,
            placeholder: None,
            min: None,
            max: None,
            disabled: false,
            width: 200.0,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    /// Validation message; overrides `help` and turns the border danger.
    pub fn error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
        self
    }

    /// Earliest selectable ISO date (inclusive).
    pub fn min(mut self, min: &'a str) -> Self {
        self.min = Some(min);
        self
    }

    /// Latest selectable ISO date (inclusive).
    pub fn max(mut self, max: &'a str) -> Self {
        self.max = Some(max);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self {
            state,
            label,
            help,
            error,
            placeholder,
            min,
            max,
            disabled,
            width,
        } = self;
        let DatePickerState { open, cal } = state;

        let inner = ui.vertical(|ui| {
            ui.set_max_width(width);
            if let Some(label) = label {
                ui.label(
                    egui::RichText::new(label)
                        .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm))
                        .color(t.fg[1]),
                );
                ui.add_space(t.space.x(1.0));
            }

            let sense = if disabled {
                Sense::hover()
            } else {
                Sense::click()
            };
            let (rect, response) = ui.allocate_exact_size(Vec2::new(width, t.control.md), sense);
            let name = label
                .or(cal.value.as_deref())
                .or(placeholder)
                .unwrap_or("date");
            response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, !disabled, name));

            let was_open = *open;
            let mut outcome = Outcome::Ignored;
            if response.clicked() {
                *open = !*open;
                outcome = Outcome::Consumed;
            }
            if was_open && ui.input(|i| i.key_pressed(Key::Escape)) {
                *open = false;
            }

            // Field chrome: well + calendar glyph + value/placeholder + chevron.
            if ui.is_rect_visible(rect) {
                let radius = CornerRadius::same(t.radius.md as u8);
                let focused = response.has_focus() || *open;
                let border = if disabled {
                    t.border.subtle
                } else if error.is_some() {
                    t.danger.base
                } else if focused {
                    t.accent.base
                } else {
                    t.border.default
                };
                ui.painter().rect_filled(rect, radius, t.bg[1]);
                ui.painter().rect_stroke(
                    rect,
                    radius,
                    Stroke::new(1.0, border),
                    StrokeKind::Inside,
                );
                util::focus_ring(ui, &response, rect, t.radius.md, &t);

                let glyph_color = if disabled { t.fg[3] } else { t.fg[2] };
                let g = util::galley(
                    ui,
                    "▦",
                    t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                    glyph_color,
                );
                ui.painter().galley(
                    egui::pos2(rect.min.x + 10.0, rect.center().y - g.size().y / 2.0),
                    g,
                    glyph_color,
                );

                let (text, color): (&str, Color32) = match cal.value.as_deref() {
                    Some(value) => (value, if disabled { t.fg[3] } else { t.fg[0] }),
                    None => (placeholder.unwrap_or("Pick a date…"), t.fg[3]),
                };
                let g = util::galley(
                    ui,
                    text,
                    t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                    color,
                );
                ui.painter().galley(
                    egui::pos2(rect.min.x + 28.0, rect.center().y - g.size().y / 2.0),
                    g,
                    color,
                );

                let chev = util::galley(
                    ui,
                    Glyph::ChevronDown.as_str(),
                    t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm),
                    glyph_color,
                );
                ui.painter().galley(
                    egui::pos2(
                        rect.max.x - 10.0 - chev.size().x,
                        rect.center().y - chev.size().y / 2.0,
                    ),
                    chev,
                    glyph_color,
                );
            }

            // Flyout.
            let mut picked = false;
            if *open && !disabled {
                let frame = egui::Frame::new()
                    .fill(t.bg[4])
                    .stroke(Stroke::new(1.0, t.border.default))
                    .corner_radius(CornerRadius::same(t.radius.md as u8))
                    .inner_margin(Margin::same(8));
                Popup::from_response(&response)
                    .open_bool(open)
                    .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                    .gap(4.0)
                    .frame(frame)
                    .show(|ui| {
                        let mut calendar = Calendar::new(cal);
                        if let Some(min) = min {
                            calendar = calendar.min(min);
                        }
                        if let Some(max) = max {
                            calendar = calendar.max(max);
                        }
                        if calendar.show(ui).changed() {
                            picked = true;
                        }
                    });
                if picked {
                    *open = false;
                }
            }

            if picked {
                outcome = Outcome::Changed;
            } else if was_open && !*open && outcome == Outcome::Ignored {
                outcome = Outcome::Cancelled;
            }

            if let Some(error) = error {
                sub_line(ui, &t, error, t.danger.base);
            } else if let Some(help) = help {
                sub_line(ui, &t, help, t.fg[2]);
            }
            (response, outcome)
        });

        let (response, outcome) = inner.inner;
        ForgeResponse::new(response, outcome)
    }
}

fn sub_line(ui: &mut Ui, t: &Theme, text: &str, color: Color32) {
    ui.add_space(t.space.x(1.0));
    ui.label(
        egui::RichText::new(text)
            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
            .color(color),
    );
}
