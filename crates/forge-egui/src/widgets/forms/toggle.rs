//! Switch toggle — a 36×20 track pill with a thumb animated on
//! `Context::animate_bool_with_time` (`t.motion.base`).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{blend, FontWeight, Theme};
use crate::widgets::util;
use egui::{Rect, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType};

const TRACK: Vec2 = Vec2::new(36.0, 20.0);
const THUMB_R: f32 = 8.0;
const GAP: f32 = 8.0;

pub struct Toggle<'a> {
    on: &'a mut bool,
    label: Option<&'a str>,
    disabled: bool,
}

impl<'a> Toggle<'a> {
    pub fn new(on: &'a mut bool) -> Toggle<'a> {
        Toggle {
            on,
            label: None,
            disabled: false,
        }
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
        let text_color = if self.disabled { t.fg[3] } else { t.fg[0] };
        let galley = self.label.map(|l| {
            util::galley(
                ui,
                l,
                t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                text_color,
            )
        });

        let label_w = galley.as_ref().map_or(0.0, |g| GAP + g.size().x);
        let height = TRACK.y.max(galley.as_ref().map_or(0.0, |g| g.size().y));
        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click()
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(TRACK.x + label_w, height), sense);

        let mut outcome = Outcome::Ignored;
        if response.clicked() && !self.disabled {
            *self.on = !*self.on;
            outcome = Outcome::Changed;
        }
        let on = *self.on;
        // No Switch in egui 0.35's WidgetType — Checkbox is the closest role.
        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                !self.disabled,
                on,
                self.label.unwrap_or_default(),
            )
        });

        if ui.is_rect_visible(rect) {
            let k = ui
                .ctx()
                .animate_bool_with_time(response.id.with("on"), on, t.motion.base);
            let track = Rect::from_min_size(
                egui::pos2(rect.min.x, rect.center().y - TRACK.y / 2.0),
                TRACK,
            );
            let pill = TRACK.y / 2.0;
            let fill = if self.disabled {
                t.bg[2]
            } else {
                blend(t.accent.base, t.bg[3], k)
            };
            ui.painter().rect_filled(track, pill, fill);
            ui.painter().rect_stroke(
                track,
                pill,
                Stroke::new(
                    1.0,
                    if self.disabled {
                        t.border.subtle
                    } else {
                        t.border.default
                    },
                ),
                StrokeKind::Inside,
            );
            util::focus_ring(ui, &response, track, pill, &t);

            let x0 = track.min.x + 2.0 + THUMB_R;
            let x1 = track.max.x - 2.0 - THUMB_R;
            let thumb = if self.disabled {
                t.fg[3]
            } else {
                blend(t.accent.contrast, t.fg[1], k)
            };
            ui.painter().circle_filled(
                egui::pos2(x0 + (x1 - x0) * k, track.center().y),
                THUMB_R,
                thumb,
            );

            if let Some(g) = galley {
                ui.painter().galley(
                    egui::pos2(
                        rect.min.x + TRACK.x + GAP,
                        rect.center().y - g.size().y / 2.0,
                    ),
                    g,
                    text_color,
                );
            }
        }

        ForgeResponse::new(response, outcome)
    }
}
