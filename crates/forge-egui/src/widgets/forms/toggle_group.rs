//! Segmented control: one row of connected segments bound to a `usize` index.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use egui::{
    CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType,
};

const PAD_X: f32 = 12.0;

pub struct ToggleGroup<'a> {
    value: &'a mut usize,
    options: &'a [&'a str],
    disabled: bool,
}

impl<'a> ToggleGroup<'a> {
    pub fn new(value: &'a mut usize, options: &'a [&'a str]) -> ToggleGroup<'a> {
        ToggleGroup {
            value,
            options,
            disabled: false,
        }
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
            disabled,
        } = self;
        let height = t.control.sm;
        let r = t.radius.md as u8;
        let sense = if disabled {
            Sense::hover()
        } else {
            Sense::click()
        };

        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mut outcome = Outcome::Ignored;
            let mut union: Option<Response> = None;
            let mut segments: Vec<(Rect, Response)> = Vec::with_capacity(options.len());

            for (i, option) in options.iter().enumerate() {
                let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
                let galley = util::galley(ui, option, font, t.fg[1]);
                let w = galley.size().x + PAD_X * 2.0;
                let (rect, response) = ui.allocate_exact_size(Vec2::new(w, height), sense);
                if response.clicked() && !disabled && *value != i {
                    *value = i;
                    outcome = Outcome::Changed;
                }
                let selected = *value == i;
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::RadioButton, !disabled, selected, option)
                });

                if ui.is_rect_visible(rect) {
                    let radius = segment_radius(r, i, options.len());
                    let fill = if selected {
                        t.accent.bg
                    } else if response.hovered() && !disabled {
                        t.bg[3]
                    } else {
                        t.bg[2]
                    };
                    ui.painter().rect_filled(rect, radius, fill);
                    let text = if disabled {
                        t.fg[3]
                    } else if selected {
                        t.accent.fg
                    } else {
                        t.fg[1]
                    };
                    let g = util::galley(
                        ui,
                        option,
                        t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm),
                        text,
                    );
                    ui.painter().galley(rect.center() - g.size() / 2.0, g, text);
                }
                segments.push((rect, response.clone()));
                union = Some(match union.take() {
                    Some(u) => u.union(response),
                    None => response,
                });
            }

            // Chrome over the fills: outer border, separators, then the
            // selected segment's accent outline on top.
            if let (Some(first), Some(last)) = (segments.first(), segments.last()) {
                let outer = first.0.union(last.0);
                let border = if disabled {
                    t.border.subtle
                } else {
                    t.border.default
                };
                ui.painter().rect_stroke(
                    outer,
                    CornerRadius::same(r),
                    Stroke::new(1.0, border),
                    StrokeKind::Inside,
                );
                for (rect, _) in segments.iter().skip(1) {
                    ui.painter()
                        .vline(rect.min.x, rect.y_range(), Stroke::new(1.0, border));
                }
                if !disabled {
                    if let Some((rect, _)) = segments.get(*value) {
                        ui.painter().rect_stroke(
                            *rect,
                            segment_radius(r, *value, options.len()),
                            Stroke::new(1.0, t.accent.base),
                            StrokeKind::Inside,
                        );
                    }
                    for (rect, response) in &segments {
                        util::focus_ring(ui, response, *rect, t.radius.md, &t);
                    }
                }
            }

            (
                union.expect("ToggleGroup needs at least one option"),
                outcome,
            )
        });

        let (response, outcome) = inner.inner;
        ForgeResponse::new(response, outcome)
    }
}

fn segment_radius(r: u8, i: usize, len: usize) -> CornerRadius {
    let first = i == 0;
    let last = i + 1 == len;
    CornerRadius {
        nw: if first { r } else { 0 },
        sw: if first { r } else { 0 },
        ne: if last { r } else { 0 },
        se: if last { r } else { 0 },
    }
}
