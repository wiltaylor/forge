//! Custom-painted slider bound to an `f64`: drag, click-to-jump, and arrow
//! keys when focused.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use egui::{Color32, EventFilter, Key, Rect, Sense, Stroke, Ui, Vec2, WidgetInfo};
use std::ops::RangeInclusive;

const TRACK_W: f32 = 180.0;
const TRACK_H: f32 = 4.0;
const THUMB_R: f32 = 7.0;

pub struct Slider<'a> {
    value: &'a mut f64,
    range: RangeInclusive<f64>,
    step: Option<f64>,
    show_value: bool,
    label: Option<&'a str>,
    disabled: bool,
}

impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f64, range: RangeInclusive<f64>) -> Slider<'a> {
        Slider {
            value,
            range,
            step: None,
            show_value: true,
            label: None,
            disabled: false,
        }
    }

    /// Snap increment; also the arrow-key step.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let (min, max) = (*self.range.start(), *self.range.end());
        let span = (max - min).max(f64::EPSILON);

        let inner = ui.horizontal(|ui| {
            if let Some(label) = self.label {
                ui.label(
                    egui::RichText::new(label)
                        .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                        .color(if self.disabled { t.fg[3] } else { t.fg[1] }),
                );
            }

            let sense = if self.disabled {
                Sense::hover()
            } else {
                Sense::click_and_drag()
            };
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(TRACK_W, THUMB_R * 2.0 + 4.0), sense);

            let x0 = rect.min.x + THUMB_R;
            let x1 = rect.max.x - THUMB_R;
            let mut v = self.value.clamp(min, max);
            let mut outcome = Outcome::Ignored;

            if !self.disabled {
                if response.dragged() || response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let ratio = ((pos.x - x0) / (x1 - x0)).clamp(0.0, 1.0) as f64;
                        v = min + ratio * span;
                    }
                }
                if response.has_focus() {
                    // Keep arrow keys on the slider instead of moving focus.
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            response.id,
                            EventFilter {
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                ..Default::default()
                            },
                        );
                    });
                    let key_step = self.step.unwrap_or(span / 20.0);
                    let (inc, dec) = ui.input(|i| {
                        (
                            i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowUp),
                            i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowDown),
                        )
                    });
                    if inc {
                        v += key_step;
                    }
                    if dec {
                        v -= key_step;
                    }
                }
                if let Some(step) = self.step {
                    if step > 0.0 {
                        v = min + ((v - min) / step).round() * step;
                    }
                }
                v = v.clamp(min, max);
                if v != *self.value {
                    *self.value = v;
                    outcome = Outcome::Changed;
                }
            }
            response.widget_info(|| {
                WidgetInfo::slider(!self.disabled, v, self.label.unwrap_or_default())
            });

            if ui.is_rect_visible(rect) {
                let cy = rect.center().y;
                let track = Rect::from_min_max(
                    egui::pos2(x0, cy - TRACK_H / 2.0),
                    egui::pos2(x1, cy + TRACK_H / 2.0),
                );
                let vx = x0 + (x1 - x0) * ((v - min) / span) as f32;
                ui.painter().rect_filled(track, TRACK_H / 2.0, t.bg[3]);
                let fill = if self.disabled {
                    t.border.strong
                } else {
                    t.accent.base
                };
                ui.painter().rect_filled(
                    Rect::from_min_max(track.min, egui::pos2(vx, track.max.y)),
                    TRACK_H / 2.0,
                    fill,
                );
                let active = response.dragged() || response.has_focus();
                let thumb_border = if self.disabled {
                    t.border.subtle
                } else if active {
                    t.accent.base
                } else {
                    t.border.strong
                };
                let thumb_fill = if self.disabled {
                    t.fg[3]
                } else {
                    Color32::WHITE
                };
                ui.painter().circle(
                    egui::pos2(vx, cy),
                    THUMB_R,
                    thumb_fill,
                    Stroke::new(1.0, thumb_border),
                );
            }

            if self.show_value {
                ui.label(
                    egui::RichText::new(fmt_value(v))
                        .font(t.mono(t.type_scale.sm))
                        .color(if self.disabled { t.fg[3] } else { t.fg[1] }),
                );
            }

            (response, outcome)
        });

        let (response, outcome) = inner.inner;
        ForgeResponse::new(response, outcome)
    }
}

fn fmt_value(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}
