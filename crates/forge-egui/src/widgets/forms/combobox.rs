//! Filterable select. The field becomes a text edit while open; the query
//! filters options by case-insensitive substring (prefix matches ranked
//! first). State transitions live in `show`; `ComboboxState` stays plain data.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::forms::field;
use crate::widgets::util;
use egui::{
    CornerRadius, Key, Margin, Popup, PopupCloseBehavior, Sense, Stroke, Ui, Vec2, WidgetInfo,
    WidgetType,
};

#[derive(Clone, Debug, Default)]
pub struct ComboboxState {
    pub open: bool,
    pub query: String,
    pub value: Option<usize>,
}

pub struct Combobox<'a> {
    state: &'a mut ComboboxState,
    options: &'a [&'a str],
    label: Option<&'a str>,
    placeholder: Option<&'a str>,
    empty_text: &'a str,
    disabled: bool,
    width: f32,
}

impl<'a> Combobox<'a> {
    pub fn new(state: &'a mut ComboboxState, options: &'a [&'a str]) -> Combobox<'a> {
        Combobox {
            state,
            options,
            label: None,
            placeholder: None,
            empty_text: "No matches",
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

    /// Shown in the flyout when the query matches nothing.
    pub fn empty_text(mut self, empty_text: &'a str) -> Self {
        self.empty_text = empty_text;
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
            empty_text,
            disabled,
            width,
        } = self;
        let ComboboxState { open, query, value } = state;

        let inner = ui.vertical(|ui| {
            ui.set_max_width(width);
            if let Some(label) = label {
                field::label_row(ui, &t, label);
            }

            let was_open = *open;
            let mut outcome = Outcome::Ignored;
            let mut committed = false;
            // One-shot flag: focus the text edit on its first open frame.
            let focus_id = ui.id().with("combobox-focus");

            // The field: a click target while closed, a text edit while open.
            let response = if *open {
                let mut prepared = egui::Frame::new()
                    .fill(t.bg[1])
                    .stroke(Stroke::new(1.0, t.accent.base))
                    .corner_radius(CornerRadius::same(t.radius.md as u8))
                    .inner_margin(Margin::symmetric(10, 0))
                    .begin(ui);
                let te_response = {
                    let ui = &mut prepared.content_ui;
                    ui.set_width(width - 20.0);
                    let hint = value.map(|i| options[i]).or(placeholder).unwrap_or("");
                    ui.add(
                        egui::TextEdit::singleline(query)
                            .frame(egui::Frame::NONE)
                            .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base))
                            .text_color(t.fg[0])
                            .hint_text(egui::RichText::new(hint).color(t.fg[3]))
                            .vertical_align(egui::Align::Center)
                            .min_size(Vec2::new(0.0, t.control.md))
                            .desired_width(ui.available_width() - 16.0),
                    )
                };
                prepared.end(ui);
                if te_response.changed() {
                    outcome = Outcome::Consumed;
                }
                if ui.ctx().data(|d| d.get_temp::<bool>(focus_id)) == Some(true) {
                    te_response.request_focus();
                    ui.ctx().data_mut(|d| d.remove::<bool>(focus_id));
                }
                if te_response.has_focus() {
                    // Keep ↑/↓ on the highlight instead of moving egui focus.
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            te_response.id,
                            egui::EventFilter {
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                ..Default::default()
                            },
                        );
                    });
                }
                te_response
            } else {
                let sense = if disabled {
                    Sense::hover()
                } else {
                    Sense::click()
                };
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(width, t.control.md), sense);
                let display = value.map(|i| options[i]);
                let name = label.or(display).or(placeholder).unwrap_or("combobox");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, !disabled, name));
                if response.clicked() {
                    *open = true;
                    query.clear();
                    outcome = Outcome::Consumed;
                    ui.ctx().data_mut(|d| d.insert_temp(focus_id, true));
                }
                if ui.is_rect_visible(rect) {
                    let border = field::well_border(&t, false, response.has_focus(), disabled);
                    field::well(ui, &t, rect, border);
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
                response
            };

            // The open branch renders the TextEdit with a different id, so
            // key the highlight on the stable vertical-container id.
            let hl_id = ui.id().with("combobox-hl");
            if *open {
                let filtered = filter(options, query);
                let (up, down, enter) = ui.input(|i| {
                    (
                        i.key_pressed(Key::ArrowUp),
                        i.key_pressed(Key::ArrowDown),
                        i.key_pressed(Key::Enter),
                    )
                });
                let mut hl = field::highlight(ui, hl_id);
                if !filtered.is_empty() {
                    hl = hl.min(filtered.len() - 1);
                    if down {
                        hl = (hl + 1).min(filtered.len() - 1);
                    }
                    if up {
                        hl = hl.saturating_sub(1);
                    }
                }
                field::set_highlight(ui, hl_id, hl);
                if enter && was_open {
                    if let Some(&idx) = filtered.get(hl) {
                        *value = Some(idx);
                        committed = true;
                    }
                    *open = false;
                }

                let popup = Popup::from_response(&response)
                    .id(response.id.with("popup"))
                    .open_bool(open)
                    .close_behavior(PopupCloseBehavior::IgnoreClicks)
                    .gap(4.0)
                    .width(width)
                    .frame(field::flyout_frame(&t))
                    .show(|ui| {
                        ui.set_min_width(width - 10.0);
                        if filtered.is_empty() {
                            ui.label(
                                egui::RichText::new(empty_text)
                                    .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                                    .color(t.fg[3]),
                            );
                        }
                        for (row, &idx) in filtered.iter().enumerate() {
                            let r = field::option_row(
                                ui,
                                &t,
                                options[idx],
                                *value == Some(idx),
                                row == hl,
                            );
                            if r.clicked() {
                                *value = Some(idx);
                                committed = true;
                            }
                        }
                    });
                // IgnoreClicks keeps clicks in the field from closing the
                // flyout; close on clicks outside both field and flyout.
                if let Some(popup) = popup {
                    let clicked_away = ui.input(|i| i.pointer.any_click())
                        && popup.response.clicked_elsewhere()
                        && !response.hovered();
                    if committed || clicked_away {
                        *open = false;
                    }
                }
            }

            if committed {
                query.clear();
                outcome = Outcome::Changed;
            } else if was_open && !*open {
                query.clear();
                outcome = Outcome::Cancelled;
            }
            (response, outcome)
        });

        let (response, outcome) = inner.inner;
        ForgeResponse::new(response, outcome)
    }
}

/// Case-insensitive substring filter; prefix matches rank first. Returns
/// indices into `options`.
fn filter(options: &[&str], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return (0..options.len()).collect();
    }
    let mut prefix = Vec::new();
    let mut contains = Vec::new();
    for (i, option) in options.iter().enumerate() {
        let lower = option.to_lowercase();
        if lower.starts_with(&q) {
            prefix.push(i);
        } else if lower.contains(&q) {
            contains.push(i);
        }
    }
    prefix.extend(contains);
    prefix
}
