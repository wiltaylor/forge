//! Runtime dialogs: confirm and the command palette, resolved through the
//! [`DialogResult`] cell pattern shared with forge-tui — `ctx.confirm(..)`
//! returns a handle the app polls with `take()` in later frames; no
//! callbacks cross the modal boundary.

use crate::theme::{scrim, FontWeight, Theme};
use crate::widgets::{Button, Variant};
use std::cell::Cell;
use std::rc::Rc;

/// A one-shot result cell bridging a modal back to the app.
pub struct DialogResult<T> {
    cell: Rc<Cell<Option<T>>>,
}

impl<T> DialogResult<T> {
    fn pair() -> (DialogResult<T>, Rc<Cell<Option<T>>>) {
        let cell = Rc::new(Cell::new(None));
        (DialogResult { cell: cell.clone() }, cell)
    }

    /// The resolved value, once. `None` while the dialog is still open (or
    /// after the value was already taken).
    pub fn take(&self) -> Option<T> {
        self.cell.take()
    }
}

/// A command-palette entry.
#[derive(Clone, Debug)]
pub struct Command {
    pub label: String,
    pub hint: Option<String>,
}

impl Command {
    pub fn new(label: impl Into<String>) -> Command {
        Command {
            label: label.into(),
            hint: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Command {
        self.hint = Some(hint.into());
        self
    }
}

enum Dialog {
    Confirm {
        title: String,
        message: String,
        verb: String,
        danger: bool,
        result: Rc<Cell<Option<bool>>>,
    },
    Palette {
        commands: Vec<Command>,
        query: String,
        highlight: usize,
        result: Rc<Cell<Option<usize>>>,
    },
}

#[derive(Default)]
pub(crate) struct DialogHost {
    stack: Vec<Dialog>,
}

impl DialogHost {
    pub(crate) fn confirm(
        &mut self,
        title: &str,
        message: &str,
        verb: &str,
        danger: bool,
    ) -> DialogResult<bool> {
        let (result, cell) = DialogResult::pair();
        self.stack.push(Dialog::Confirm {
            title: title.to_owned(),
            message: message.to_owned(),
            verb: verb.to_owned(),
            danger,
            result: cell,
        });
        result
    }

    pub(crate) fn palette(&mut self, commands: Vec<Command>) -> DialogResult<usize> {
        let (result, cell) = DialogResult::pair();
        self.stack.push(Dialog::Palette {
            commands,
            query: String::new(),
            highlight: 0,
            result: cell,
        });
        result
    }

    pub(crate) fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Paint the top dialog (modal: scrim + centered panel). Returns nothing;
    /// resolved dialogs pop themselves.
    pub(crate) fn show(&mut self, ctx: &egui::Context, theme: &Theme) {
        let Some(dialog) = self.stack.last_mut() else {
            return;
        };

        // Scrim over everything below the dialog.
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::new("forge-dialog-scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                ui.allocate_response(screen.size(), egui::Sense::click());
                ui.painter().rect_filled(screen, 0.0, scrim(theme));
            });

        let mut done = false;
        egui::Area::new(egui::Id::new("forge-dialog"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, -40.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme.bg[4])
                    .stroke(egui::Stroke::new(1.0, theme.border.default))
                    .corner_radius(egui::CornerRadius::same(theme.radius.lg as u8))
                    .inner_margin(egui::Margin::same(20))
                    .show(ui, |ui| match dialog {
                        Dialog::Confirm {
                            title,
                            message,
                            verb,
                            danger,
                            result,
                        } => {
                            ui.set_width(400.0);
                            ui.label(
                                egui::RichText::new(title.as_str())
                                    .font(t_font(ui, theme))
                                    .color(theme.fg[0]),
                            );
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(message.as_str()).color(theme.fg[1]));
                            ui.add_space(16.0);
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let variant = if *danger {
                                            Variant::Danger
                                        } else {
                                            Variant::Primary
                                        };
                                        if Button::new(verb).variant(variant).show(ui).clicked()
                                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            result.set(Some(true));
                                            done = true;
                                        }
                                        if Button::new("Cancel")
                                            .variant(Variant::Ghost)
                                            .show(ui)
                                            .clicked()
                                        {
                                            result.set(Some(false));
                                            done = true;
                                        }
                                    },
                                );
                            });
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                result.set(Some(false));
                                done = true;
                            }
                        }
                        Dialog::Palette {
                            commands,
                            query,
                            highlight,
                            result,
                        } => {
                            ui.set_width(480.0);
                            let edit = egui::TextEdit::singleline(query)
                                .hint_text("Type a command…")
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE);
                            let response = ui.add(edit);
                            response.request_focus();
                            ui.add_space(6.0);
                            crate::widgets::Separator::new().spacing(0.0).show(ui);
                            ui.add_space(6.0);

                            let q = query.to_lowercase();
                            let filtered: Vec<(usize, &Command)> = commands
                                .iter()
                                .enumerate()
                                .filter(|(_, c)| c.label.to_lowercase().contains(&q))
                                .collect();
                            *highlight = (*highlight).min(filtered.len().saturating_sub(1));

                            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                *highlight = (*highlight + 1).min(filtered.len().saturating_sub(1));
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                *highlight = highlight.saturating_sub(1);
                            }

                            for (row, (idx, command)) in filtered.iter().enumerate() {
                                let selected = row == *highlight;
                                let fill = if selected {
                                    theme.accent.bg
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                let text = if selected {
                                    theme.accent.fg
                                } else {
                                    theme.fg[1]
                                };
                                let r = egui::Frame::new()
                                    .fill(fill)
                                    .corner_radius(egui::CornerRadius::same(theme.radius.sm as u8))
                                    .inner_margin(egui::Margin::symmetric(8, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&command.label).color(text),
                                            );
                                            if let Some(hint) = &command.hint {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(hint.as_str())
                                                                .size(theme.type_scale.sm)
                                                                .color(theme.fg[2]),
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                    })
                                    .response;
                                if r.interact(egui::Sense::click()).clicked() {
                                    result.set(Some(*idx));
                                    done = true;
                                }
                            }
                            if filtered.is_empty() {
                                ui.label(
                                    egui::RichText::new("No matches")
                                        .color(theme.fg[3])
                                        .size(theme.type_scale.sm),
                                );
                            }

                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if let Some((idx, _)) = filtered.get(*highlight) {
                                    result.set(Some(*idx));
                                    done = true;
                                }
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                done = true; // resolves to None — cell stays empty
                            }
                        }
                    });
            });

        if done {
            self.stack.pop();
        }
    }
}

fn t_font(ui: &egui::Ui, theme: &Theme) -> egui::FontId {
    theme.font(ui.ctx(), FontWeight::SemiBold, theme.type_scale.md)
}
