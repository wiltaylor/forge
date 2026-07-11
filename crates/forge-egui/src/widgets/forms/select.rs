//! Dropdown select with explicit state: `SelectState` is plain app-owned
//! data; the flyout rides `egui::Popup` (click-away/Esc close), and the
//! keyboard highlight cursor lives in egui temp memory keyed by the field id.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::forms::field;
use crate::widgets::util;
use egui::{Key, Popup, PopupCloseBehavior, Sense, Ui, Vec2, WidgetInfo, WidgetType};

#[derive(Clone, Debug, Default)]
pub struct SelectState {
    pub open: bool,
    pub value: Option<usize>,
}

pub struct Select<'a> {
    state: &'a mut SelectState,
    options: &'a [&'a str],
    label: Option<&'a str>,
    placeholder: Option<&'a str>,
    disabled: bool,
    width: f32,
}

impl<'a> Select<'a> {
    pub fn new(state: &'a mut SelectState, options: &'a [&'a str]) -> Select<'a> {
        Select {
            state,
            options,
            label: None,
            placeholder: None,
            disabled: false,
            width: 200.0,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = Some(placeholder);
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
            options,
            label,
            placeholder,
            disabled,
            width,
        } = self;
        let SelectState { open, value } = state;

        let inner = ui.vertical(|ui| {
            ui.set_max_width(width);
            if let Some(label) = label {
                field::label_row(ui, &t, label);
            }

            let sense = if disabled {
                Sense::hover()
            } else {
                Sense::click()
            };
            let (rect, response) = ui.allocate_exact_size(Vec2::new(width, t.control.md), sense);
            let display = value.map(|i| options[i]);
            let name = label.or(display).or(placeholder).unwrap_or("select");
            response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, !disabled, name));

            let was_open = *open;
            let hl_id = response.id.with("hl");
            let (up, down, enter) = if was_open {
                ui.input(|i| {
                    (
                        i.key_pressed(Key::ArrowUp),
                        i.key_pressed(Key::ArrowDown),
                        i.key_pressed(Key::Enter),
                    )
                })
            } else {
                (false, false, false)
            };

            let mut outcome = Outcome::Ignored;
            let mut committed = false;
            if was_open && enter && !options.is_empty() {
                *value = Some(field::highlight(ui, hl_id).min(options.len() - 1));
                *open = false;
                committed = true;
            } else if response.clicked() {
                *open = !*open;
                outcome = Outcome::Consumed;
                if *open {
                    field::set_highlight(ui, hl_id, value.unwrap_or(0));
                }
            }
            if *open && response.has_focus() {
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
            if was_open && !options.is_empty() {
                let mut hl = field::highlight(ui, hl_id).min(options.len() - 1);
                if down {
                    hl = (hl + 1).min(options.len() - 1);
                }
                if up {
                    hl = hl.saturating_sub(1);
                }
                field::set_highlight(ui, hl_id, hl);
            }

            // Field paint.
            if ui.is_rect_visible(rect) {
                let focused = response.has_focus() || *open;
                field::well(
                    ui,
                    &t,
                    rect,
                    field::well_border(&t, false, focused, disabled),
                );
                util::focus_ring(ui, &response, rect, t.radius.md, &t);
                let (text, color) = match display {
                    Some(text) => (text, if disabled { t.fg[3] } else { t.fg[0] }),
                    None => (placeholder.unwrap_or(""), t.fg[3]),
                };
                let g = util::galley(
                    ui,
                    text,
                    t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                    color,
                );
                ui.painter().galley(
                    egui::pos2(rect.min.x + 10.0, rect.center().y - g.size().y / 2.0),
                    g,
                    color,
                );
                field::chevron(ui, &t, rect, if disabled { t.fg[3] } else { t.fg[2] });
            }

            // Flyout.
            if *open {
                let hl = field::highlight(ui, hl_id);
                Popup::from_response(&response)
                    .open_bool(open)
                    .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                    .gap(4.0)
                    .width(width)
                    .frame(field::flyout_frame(&t))
                    .show(|ui| {
                        ui.set_min_width(width - 10.0);
                        for (i, option) in options.iter().enumerate() {
                            let row = field::option_row(ui, &t, option, *value == Some(i), i == hl);
                            if row.clicked() {
                                *value = Some(i);
                                committed = true;
                            }
                        }
                    });
                if committed {
                    *open = false;
                }
            }

            if committed {
                outcome = Outcome::Changed;
            } else if was_open && !*open {
                outcome = Outcome::Cancelled;
            }
            (response, outcome)
        });

        let (response, outcome) = inner.inner;
        ForgeResponse::new(response, outcome)
    }
}
