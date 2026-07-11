//! Always-visible option list, single- or multi-select, in a scrollable well.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{CornerRadius, Key, Margin, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

#[derive(Clone, Debug, Default)]
pub struct ListBoxState {
    pub selected: Vec<usize>,
    pub highlight: usize,
}

pub struct ListBox<'a> {
    state: &'a mut ListBoxState,
    options: &'a [&'a str],
    multiple: bool,
    height: Option<f32>,
}

impl<'a> ListBox<'a> {
    pub fn new(state: &'a mut ListBoxState, options: &'a [&'a str]) -> ListBox<'a> {
        ListBox {
            state,
            options,
            multiple: false,
            height: None,
        }
    }

    /// Multi-select: clicking toggles membership instead of replacing it.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Visible height before scrolling (default ≈ 5 rows).
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self {
            state,
            options,
            multiple,
            height,
        } = self;
        let row_h = t.control.sm;
        let height = height.unwrap_or(row_h * 5.0);

        let inner = egui::Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.default))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin::same(4))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("forge-listbox", multiple, options.len()))
                    .max_height(height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let mut outcome = Outcome::Ignored;
                        let mut union: Option<Response> = None;
                        let mut responses = Vec::with_capacity(options.len());

                        for (i, option) in options.iter().enumerate() {
                            let response = row_ui(
                                ui,
                                &t,
                                option,
                                state.selected.contains(&i),
                                state.highlight == i,
                                multiple,
                                row_h,
                            );
                            if response.clicked() {
                                toggle(state, i, multiple);
                                outcome = Outcome::Changed;
                            }
                            responses.push(response.clone());
                            union = Some(match union.take() {
                                Some(u) => u.union(response),
                                None => response,
                            });
                        }

                        // ↑/↓ move the highlight (and focus) while a row is
                        // focused; Space/Enter activate via egui's clicked().
                        if let Some(fi) = responses.iter().position(|r| r.has_focus()) {
                            ui.memory_mut(|m| {
                                m.set_focus_lock_filter(
                                    responses[fi].id,
                                    egui::EventFilter {
                                        vertical_arrows: true,
                                        ..Default::default()
                                    },
                                );
                            });
                            let (up, down) = ui.input(|i| {
                                (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown))
                            });
                            let next = if down {
                                (fi + 1).min(options.len().saturating_sub(1))
                            } else if up {
                                fi.saturating_sub(1)
                            } else {
                                fi
                            };
                            state.highlight = next;
                            if next != fi {
                                responses[next].request_focus();
                            }
                        }

                        (union, outcome)
                    })
                    .inner
            })
            .inner;

        let (union, outcome) = inner;
        let response = union.expect("ListBox needs at least one option");
        ForgeResponse::new(response, outcome)
    }
}

fn toggle(state: &mut ListBoxState, i: usize, multiple: bool) {
    state.highlight = i;
    if multiple {
        if let Some(pos) = state.selected.iter().position(|&s| s == i) {
            state.selected.remove(pos);
        } else {
            state.selected.push(i);
        }
    } else {
        state.selected.clear();
        state.selected.push(i);
    }
}

fn row_ui(
    ui: &mut Ui,
    t: &Theme,
    text: &str,
    selected: bool,
    highlighted: bool,
    multiple: bool,
    row_h: f32,
) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, text));

    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(t.radius.sm as u8);
        if selected {
            ui.painter().rect_filled(rect, radius, t.accent.bg);
        } else if response.hovered() || highlighted || response.has_focus() {
            ui.painter().rect_filled(rect, radius, t.bg[2]);
        }
        let color = if selected {
            t.accent.fg
        } else if response.hovered() {
            t.fg[0]
        } else {
            t.fg[1]
        };
        let g = util::galley(
            ui,
            text,
            t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
            color,
        );
        ui.painter().galley(
            egui::pos2(rect.min.x + 8.0, rect.center().y - g.size().y / 2.0),
            g,
            color,
        );
        if selected && multiple {
            let g = util::galley(
                ui,
                Glyph::Check.as_str(),
                t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                t.accent.fg,
            );
            ui.painter().galley(
                egui::pos2(
                    rect.max.x - 8.0 - g.size().x,
                    rect.center().y - g.size().y / 2.0,
                ),
                g,
                t.accent.fg,
            );
        }
    }
    response
}
