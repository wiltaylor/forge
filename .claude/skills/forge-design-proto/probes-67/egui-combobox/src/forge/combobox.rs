//! combobox — egui.
//!
//! A single-select field over a list too long to scan, where the user narrows
//! by typing. The field is two different widgets: closed it is a click target
//! painted to look like an input, open it is a real `TextEdit` over `query`.
//!
//! `query` is a `String`, not an optional — egui has no unset. The
//! unset-versus-empty distinction is recovered with hint text: while `query` is
//! empty, the selected option's label shows as the hint.

use std::sync::Arc;

use egui::{
    text::{CCursor, CCursorRange, LayoutJob, TextWrapping},
    Align, Color32, CornerRadius, FontId, Frame, Galley, Id, Key, Margin, Modifiers, Popup,
    PopupCloseBehavior, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
    WidgetInfo, WidgetType,
};

use super::{
    icon,
    response::{ForgeResponse, Outcome},
    theme::{FontWeight, Theme},
};

/// One option. `label` is what the user reads and what the filter matches.
#[derive(Clone, Copy, Debug)]
pub struct ComboBoxOption<'a> {
    pub label: &'a str,
    pub disabled: bool,
}

impl<'a> ComboBoxOption<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            disabled: false,
        }
    }

    /// A disabled option shows, and cannot be committed.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a> From<&'a str> for ComboBoxOption<'a> {
    fn from(label: &'a str) -> Self {
        Self::new(label)
    }
}

/// The state the caller owns. It has no methods; every transition happens in
/// `ComboBox::show`.
///
/// `active` — which option the keyboard is on — is not here. It is derived
/// state, and it lives in egui memory beside the widget id.
#[derive(Clone, Debug, Default)]
pub struct ComboBoxState {
    /// The popup is showing.
    pub open: bool,
    /// What the user typed. Empty means "show the selected label as the hint".
    pub query: String,
    /// The committed selection, as an index into the options slice.
    pub value: Option<usize>,
}

/// The `combobox` control.
pub struct ComboBox<'a> {
    id_salt: Id,
    state: &'a mut ComboBoxState,
    options: &'a [ComboBoxOption<'a>],
    label: Option<&'a str>,
    help: Option<&'a str>,
    placeholder: &'a str,
    empty_text: &'a str,
    width: Option<f32>,
    disabled: bool,
    max_visible: usize,
}

impl<'a> ComboBox<'a> {
    /// `id_salt` must be unique within the parent `Ui`.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        state: &'a mut ComboBoxState,
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            state,
            options: &[],
            label: None,
            help: None,
            placeholder: "",
            empty_text: "",
            width: None,
            disabled: false,
            max_visible: 8,
        }
    }

    /// The options, in the order they show.
    pub fn options(mut self, options: &'a [ComboBoxOption<'a>]) -> Self {
        self.options = options;
        self
    }

    /// The field label, above the field. Leave it unset inside a
    /// `settings-row`, which carries the label itself.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Help text, under the field.
    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    /// What the field shows while nothing is selected.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// The single line the popup holds when the filter matches nothing. Say
    /// what to do next.
    pub fn empty_text(mut self, empty_text: &'a str) -> Self {
        self.empty_text = empty_text;
        self
    }

    /// Field width. The popup takes the same width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// How many options show before the list scrolls.
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        self.max_visible = max_visible;
        self
    }

    /// Paint the control, run every transition, and report what happened.
    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let enabled = !self.disabled;
        ui.add_enabled_ui(enabled, |ui| self.show_enabled(ui)).inner
    }

    fn show_enabled(self, ui: &mut Ui) -> ForgeResponse {
        let Self {
            id_salt,
            state,
            options,
            label,
            help,
            placeholder,
            empty_text,
            width,
            disabled,
            max_visible,
        } = self;

        let theme = Theme::get(ui.ctx());
        let ctx = ui.ctx().clone();

        let base = ui.make_persistent_id(id_salt);
        let field_id = base.with("field");
        let text_id = base.with("edit");
        let active_id = base.with("active");
        let focus_id = base.with("combobox-focus");
        let restore_id = base.with("combobox-restore");
        let quiet_id = base.with("combobox-quiet");

        let body = theme.font(&ctx, FontWeight::Regular, theme.type_scale.base);
        let small = theme.font(&ctx, FontWeight::Regular, theme.type_scale.sm);
        let row_h = ctx.fonts_mut(|f| f.row_height(&body));
        let width = width.unwrap_or_else(|| ui.available_width());

        let active_at_entry: i32 = ui.ctx().data(|d| d.get_temp(active_id)).unwrap_or(-1);
        let mut active = active_at_entry;
        let mut outcome = Outcome::Ignored;

        // A focus grant we asked for ourselves must not re-open the popup.
        let quiet: bool = ui.ctx().data(|d| d.get_temp(quiet_id)).unwrap_or(false);
        if quiet {
            ui.ctx().data_mut(|d| d.insert_temp(quiet_id, false));
        }

        // ---- keys, before the text edit sees them -------------------------
        let edit_focused = ui.ctx().memory(|m| m.has_focus(text_id));
        let field_focused = ui.ctx().memory(|m| m.has_focus(field_id));
        let mut filtered = filter(options, &state.query);

        if !disabled && (edit_focused || field_focused) {
            let (up, down, enter, escape) = ui.ctx().input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(Modifiers::NONE, Key::Enter),
                    i.consume_key(Modifiers::NONE, Key::Escape),
                )
            });

            if escape && state.open {
                dismiss(state, &mut active);
                ui.ctx().data_mut(|d| d.insert_temp(restore_id, true));
                outcome = outcome.merge(Outcome::Cancelled);
            }

            if down {
                if state.open {
                    if !filtered.is_empty() {
                        active = (active + 1).min(filtered.len() as i32 - 1);
                    }
                } else {
                    open(state, &mut active, options, ui, focus_id, field_id);
                    if active < 0 && !filtered.is_empty() {
                        active = 0;
                    }
                }
                outcome = outcome.merge(Outcome::Consumed);
            }

            if up && state.open && !filtered.is_empty() {
                active = (active - 1).max(0);
                outcome = outcome.merge(Outcome::Consumed);
            }

            if enter {
                if state.open {
                    let row = active;
                    if commit(state, &mut active, options, &filtered, row) {
                        ui.ctx().data_mut(|d| d.insert_temp(restore_id, true));
                        outcome = outcome.merge(Outcome::Submitted);
                    } else {
                        outcome = outcome.merge(Outcome::Consumed);
                    }
                } else {
                    open(state, &mut active, options, ui, focus_id, field_id);
                    outcome = outcome.merge(Outcome::Consumed);
                }
            }
        }

        // ---- the field ----------------------------------------------------
        let hint = state
            .value
            .and_then(|value| options.get(value))
            .map(|option| option.label)
            .unwrap_or(placeholder);
        let closed_color = if state.value.is_some() {
            theme.fg[0]
        } else {
            theme.fg[3]
        };

        let vpad = ((theme.control.md - row_h) / 2.0).max(0.0).round() as i8;
        let stroke = if state.open {
            Stroke::new(1.0, theme.accent.base)
        } else {
            Stroke::new(1.0, theme.border.default)
        };
        let frame = Frame::new()
            .fill(theme.bg[1])
            .stroke(stroke)
            .corner_radius(CornerRadius::same(theme.radius.md as u8))
            .inner_margin(Margin::symmetric(10, vpad));

        let mut typed = false;
        let mut elided = false;
        let column = ui.vertical(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.y = theme.space.x(1.0);

            if let Some(label) = label {
                ui.label(RichText::new(label).font(small.clone()).color(theme.fg[1]));
            }

            let field = frame.show(ui, |ui| {
                ui.set_width(width - 20.0);
                ui.set_height(row_h);
                ui.spacing_mut().item_spacing.x = 0.0;

                ui.horizontal(|ui| {
                    let (slot, _) = ui.allocate_exact_size(
                        Vec2::new(icon::ICON_SIZE + theme.space.x(2.0), row_h),
                        Sense::hover(),
                    );
                    icon::search(
                        ui.painter(),
                        glyph_rect(slot, Align::Min),
                        if state.open { theme.fg[1] } else { theme.fg[2] },
                    );

                    let text_w = ui.available_width() - icon::ICON_SIZE;
                    if state.open {
                        let output = TextEdit::singleline(&mut state.query)
                            .id(text_id)
                            .frame(Frame::NONE)
                            .margin(Margin::ZERO)
                            .desired_width(text_w)
                            .font(body.clone())
                            .text_color(theme.fg[0])
                            .hint_text(
                                RichText::new(hint.to_owned())
                                    .font(body.clone())
                                    .color(theme.fg[3]),
                            )
                            .show(ui);
                        typed = output.response.response.changed();

                        // Focus does not survive the widget swap: the widget
                        // that held it no longer exists. Take it back on the
                        // first frame the edit exists, and select what is
                        // there, so typing replaces it.
                        let wants: bool = ui.ctx().data(|d| d.get_temp(focus_id)).unwrap_or(false);
                        if wants {
                            output.response.response.request_focus();
                            let mut edit_state = output.state.clone();
                            let end = state.query.chars().count();
                            edit_state.cursor.set_char_range(Some(CCursorRange::two(
                                CCursor::new(0),
                                CCursor::new(end),
                            )));
                            edit_state.store(ui.ctx(), text_id);
                            ui.ctx().data_mut(|d| d.insert_temp(focus_id, false));
                        }
                    } else {
                        let (slot, _) =
                            ui.allocate_exact_size(Vec2::new(text_w, row_h), Sense::hover());
                        let galley = truncate(ui, hint, body.clone(), closed_color, text_w);
                        elided = galley.elided;
                        let pos = slot.left_center() - Vec2::new(0.0, galley.size().y / 2.0);
                        ui.painter().galley(pos, galley, closed_color);
                    }

                    let (slot, _) =
                        ui.allocate_exact_size(Vec2::new(icon::ICON_SIZE, row_h), Sense::hover());
                    icon::chevron_down(ui.painter(), glyph_rect(slot, Align::Max), theme.fg[2]);
                });
            });

            field.response
        });

        let anchor = column.inner;
        let field_rect = anchor.rect;

        // The click target exists on every frame, open or closed, so focus
        // never points at a widget that this frame did not draw. It senses
        // clicks only while closed; open, the text edit owns them.
        let mut field_response = ui.interact(
            field_rect,
            field_id,
            if state.open || disabled {
                Sense::hover()
            } else {
                Sense::click()
            },
        );

        // A truncated value stays reachable.
        if elided {
            field_response = field_response.on_hover_text(hint);
        }

        let opened_by_focus = field_response.gained_focus() && !quiet;
        if !disabled && !state.open && (field_response.clicked() || opened_by_focus) {
            open(state, &mut active, options, ui, focus_id, field_id);
            outcome = outcome.merge(Outcome::Consumed);
        }

        // Typing sets query, opens the popup, and sets active to 0.
        if typed {
            state.open = true;
            active = 0;
            filtered = filter(options, &state.query);
            outcome = outcome.merge(Outcome::Changed);
        }
        if active >= filtered.len() as i32 {
            active = filtered.len() as i32 - 1;
        }

        // The focus ring is a 2px accent stroke outside the rect.
        if ui.ctx().memory(|m| m.has_focus(text_id) || m.has_focus(field_id)) {
            ui.painter().rect_stroke(
                field_rect,
                CornerRadius::same(theme.radius.md as u8),
                Stroke::new(2.0, theme.accent.base),
                StrokeKind::Outside,
            );
        }

        if let Some(help) = help {
            ui.add_space(theme.space.x(1.0));
            ui.label(RichText::new(help).font(small).color(theme.fg[2]));
        }

        // ---- the popup ----------------------------------------------------
        let active_now = active;
        let active_changed = active_at_entry != active_now;
        let mut popup_open = state.open;

        if state.open {
            let popup_frame = Frame::new()
                .fill(theme.bg[4])
                .stroke(Stroke::new(1.0, theme.border.default))
                .corner_radius(CornerRadius::same(theme.radius.md as u8))
                .inner_margin(Margin::same(theme.space.x(1.0) as i8));

            let selected = state.value;
            let hit = Popup::from_response(&anchor)
                .id(base.with("popup"))
                .open_bool(&mut popup_open)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .gap(theme.space.x(1.0))
                .width(width)
                .frame(popup_frame)
                .show(|ui| {
                    ui.set_width(width - theme.space.x(2.0) - 2.0);
                    ui.spacing_mut().item_spacing.y = 0.0;

                    if filtered.is_empty() {
                        let (rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), theme.control.md),
                            Sense::hover(),
                        );
                        let galley = truncate(
                            ui,
                            empty_text,
                            body.clone(),
                            theme.fg[2],
                            rect.width() - theme.space.x(4.0),
                        );
                        let pos = rect.left_center()
                            + Vec2::new(theme.space.x(2.0), -galley.size().y / 2.0);
                        ui.painter().galley(pos, galley, theme.fg[2]);
                        return None;
                    }

                    ScrollArea::vertical()
                        .max_height(theme.control.md * max_visible as f32)
                        .show(ui, |ui| {
                            let mut hit = None;
                            for (row, &index) in filtered.iter().enumerate() {
                                let option = &options[index];
                                let sense = if option.disabled {
                                    Sense::hover()
                                } else {
                                    Sense::click()
                                };
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), theme.control.md),
                                    sense,
                                );
                                let is_active = active_now == row as i32;
                                let radius = CornerRadius::same(theme.radius.sm as u8);

                                if is_active {
                                    ui.painter().rect(
                                        rect,
                                        radius,
                                        theme.bg[3],
                                        Stroke::new(1.0, theme.accent.base),
                                        StrokeKind::Inside,
                                    );
                                } else if response.hovered() {
                                    ui.painter().rect_filled(rect, radius, theme.bg[2]);
                                }

                                let color = if option.disabled {
                                    theme.fg[3]
                                } else {
                                    theme.fg[0]
                                };
                                let text_w =
                                    rect.width() - theme.space.x(4.0) - icon::ICON_SIZE;
                                let galley =
                                    truncate(ui, option.label, body.clone(), color, text_w);
                                let elided = galley.elided;
                                let pos = rect.left_center()
                                    + Vec2::new(theme.space.x(2.0), -galley.size().y / 2.0);
                                ui.painter().galley(pos, galley, color);

                                if selected == Some(index) {
                                    let slot = Rect::from_center_size(
                                        egui::pos2(
                                            rect.right()
                                                - theme.space.x(2.0)
                                                - icon::ICON_SIZE / 2.0,
                                            rect.center().y,
                                        ),
                                        Vec2::splat(icon::ICON_SIZE),
                                    );
                                    icon::check(ui.painter(), slot, theme.accent.base);
                                }

                                response.widget_info(|| {
                                    WidgetInfo::selected(
                                        WidgetType::SelectableLabel,
                                        !option.disabled,
                                        selected == Some(index),
                                        option.label,
                                    )
                                });
                                if elided {
                                    response.clone().on_hover_text(option.label);
                                }
                                if response.clicked() && !option.disabled {
                                    hit = Some(row as i32);
                                }
                                if is_active && active_changed {
                                    ui.scroll_to_rect(rect, Some(Align::Center));
                                }
                            }
                            hit
                        })
                        .inner
                })
                .and_then(|inner| inner.inner);

            if let Some(row) = hit {
                if commit(state, &mut active, options, &filtered, row) {
                    popup_open = false;
                    ui.ctx().data_mut(|d| d.insert_temp(restore_id, true));
                    outcome = outcome.merge(Outcome::Submitted);
                }
            }
        }

        // Dismissing by any other route behaves as Escape.
        if state.open && !popup_open {
            dismiss(state, &mut active);
            ui.ctx().data_mut(|d| d.insert_temp(restore_id, true));
            outcome = outcome.merge(Outcome::Cancelled);
        }

        // Focus returns to whatever opened the popup, never to nothing.
        let restore: bool = ui.ctx().data(|d| d.get_temp(restore_id)).unwrap_or(false);
        if restore && !state.open {
            ui.ctx().memory_mut(|m| m.request_focus(field_id));
            ui.ctx().data_mut(|d| {
                d.insert_temp(restore_id, false);
                d.insert_temp(quiet_id, true);
            });
        }

        ui.ctx().data_mut(|d| d.insert_temp(active_id, active));

        let selected_label = state
            .value
            .and_then(|value| options.get(value))
            .map(|option| option.label)
            .unwrap_or(placeholder);
        field_response
            .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, !disabled, selected_label));

        ForgeResponse::new(field_response, outcome)
    }
}

/// The Contract's filter: a case-insensitive substring match on the label, and
/// no ranking.
fn filter(options: &[ComboBoxOption<'_>], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..options.len()).collect();
    }
    let needle = query.to_lowercase();
    options
        .iter()
        .enumerate()
        .filter(|(_, option)| option.label.to_lowercase().contains(&needle))
        .map(|(index, _)| index)
        .collect()
}

/// Open the popup, and put the keyboard on the committed option.
///
/// The closed field stops existing on the next frame, so it gives up focus
/// now. Leave focus on a widget that the next frame does not draw and the
/// accessibility tree names a node that is not there.
fn open(
    state: &mut ComboBoxState,
    active: &mut i32,
    options: &[ComboBoxOption<'_>],
    ui: &Ui,
    focus_id: Id,
    field_id: Id,
) {
    state.open = true;
    let filtered = filter(options, &state.query);
    *active = state
        .value
        .and_then(|value| filtered.iter().position(|&index| index == value))
        .map(|row| row as i32)
        .unwrap_or(-1);
    ui.ctx().data_mut(|d| d.insert_temp(focus_id, true));
    ui.ctx().memory_mut(|m| m.surrender_focus(field_id));
}

/// Escape, and every other dismissal. The committed value does not change.
fn dismiss(state: &mut ComboBoxState, active: &mut i32) {
    state.open = false;
    state.query.clear();
    *active = -1;
}

/// Commit the option at `row`. A disabled option is a no-op, and the popup
/// stays open.
fn commit(
    state: &mut ComboBoxState,
    active: &mut i32,
    options: &[ComboBoxOption<'_>],
    filtered: &[usize],
    row: i32,
) -> bool {
    if row < 0 {
        return false;
    }
    let Some(&index) = filtered.get(row as usize) else {
        return false;
    };
    if options[index].disabled {
        return false;
    }
    state.value = Some(index);
    state.open = false;
    state.query.clear();
    *active = -1;
    true
}

/// The glyph box, pinned to one end of its slot.
fn glyph_rect(slot: Rect, align: Align) -> Rect {
    let x = match align {
        Align::Min => slot.left() + icon::ICON_SIZE / 2.0,
        _ => slot.right() - icon::ICON_SIZE / 2.0,
    };
    Rect::from_center_size(egui::pos2(x, slot.center().y), Vec2::splat(icon::ICON_SIZE))
}

/// One row, truncated with an ellipsis at the end.
fn truncate(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping {
        max_width: max_width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}
