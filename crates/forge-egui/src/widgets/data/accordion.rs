//! Disclosure containers: [`Collapsible`] (single panel, self-stored open
//! flag) and [`Accordion`] (exclusive set — at most one panel open, state
//! app-owned).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Surface, TextRole, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{CornerRadius, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

/// A single disclosure panel with Forge chrome. The open flag lives in egui
/// memory keyed by the title — for app-controlled state use [`Accordion`].
pub struct Collapsible<'a> {
    title: &'a str,
    default_open: bool,
}

impl<'a> Collapsible<'a> {
    pub fn new(title: &'a str) -> Collapsible<'a> {
        Collapsible {
            title,
            default_open: false,
        }
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn show(self, ui: &mut Ui, body: impl FnOnce(&mut Ui)) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let id = ui.make_persistent_id(("forge-collapsible", self.title));
        let mut open = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or(id, self.default_open));

        let resp = header_row(ui, &t, self.title, open, None);
        let mut outcome = Outcome::Ignored;
        if resp.clicked() {
            open = !open;
            ui.ctx().data_mut(|d| d.insert_temp(id, open));
            outcome = Outcome::Changed;
        }
        if open {
            ui.horizontal(|ui| {
                ui.add_space(t.space.x(6.0));
                ui.vertical(|ui| body(ui));
            });
            ui.add_space(t.space.x(1.0));
        }
        ForgeResponse::new(resp, outcome)
    }
}

/// Exclusive-open state for [`Accordion`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccordionState {
    pub open: Option<usize>,
}

/// A stack of titled panels where opening one closes the others.
pub struct Accordion<'a> {
    state: &'a mut AccordionState,
    titles: &'a [&'a str],
}

impl<'a> Accordion<'a> {
    pub fn new(state: &'a mut AccordionState, titles: &'a [&'a str]) -> Accordion<'a> {
        Accordion { state, titles }
    }

    /// `body` is called once for the open panel with its index.
    pub fn show(self, ui: &mut Ui, mut body: impl FnMut(usize, &mut Ui)) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self { state, titles } = self;
        let mut outcome = Outcome::Ignored;
        let mut union: Option<Response> = None;

        for (i, title) in titles.iter().enumerate() {
            let open = state.open == Some(i);
            let resp = header_row(ui, &t, title, open, Some((i + 1, titles.len())));
            if resp.clicked() {
                state.open = if open { None } else { Some(i) };
                outcome = Outcome::Changed;
            }
            let open = state.open == Some(i);
            if open {
                ui.horizontal(|ui| {
                    ui.add_space(t.space.x(6.0));
                    ui.vertical(|ui| body(i, ui));
                });
                ui.add_space(t.space.x(1.0));
            }
            union = Some(match union.take() {
                Some(u) => u.union(resp.clone()),
                None => resp,
            });
        }

        let response = union.expect("Accordion needs at least one panel");
        ForgeResponse::new(response, outcome)
    }
}

/// Shared header chrome: a [`Surface::Card`] row that hovers to
/// [`Surface::Hover`], a rotating chevron + title.
fn header_row(
    ui: &mut Ui,
    t: &Theme,
    title: &str,
    open: bool,
    position: Option<(usize, usize)>,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), t.control.md),
        Sense::click(),
    );
    resp.widget_info(|| WidgetInfo::selected(WidgetType::CollapsingHeader, true, open, title));
    let _ = position;
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(t.radius.md as u8);
        let fill = if resp.hovered() {
            t.surface(Surface::Hover)
        } else {
            t.surface(Surface::Card)
        };
        ui.painter().rect_filled(rect, radius, fill);
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, t.border.subtle),
            egui::StrokeKind::Inside,
        );
        util::focus_ring(ui, &resp, rect, t.radius.md, t);

        // Chevron rotates ▸ → ▾ via ctx.animate.
        let angle = std::f32::consts::FRAC_PI_2
            * ui.ctx()
                .animate_bool_with_time(resp.id.with("chev"), open, t.motion.base);
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm);
        let g = util::galley(
            ui,
            Glyph::ChevronRight.as_str(),
            font,
            t.text(TextRole::Tertiary),
        );
        let chev_center = egui::pos2(rect.min.x + 16.0, rect.center().y);
        let chev_rect = Rect::from_center_size(chev_center, g.size());
        let mut shape = egui::epaint::TextShape::new(chev_rect.min, g, t.text(TextRole::Tertiary));
        shape.angle = angle;
        // Rotate around the glyph center: TextShape rotates around pos, so
        // offset pos to keep the glyph visually centered.
        let offset = chev_center - chev_rect.min;
        let rot = emath_rotate(offset, angle);
        shape.pos = chev_center - rot;
        ui.painter().add(shape);

        let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base);
        let color = if resp.hovered() {
            t.text(TextRole::Primary)
        } else {
            t.text(TextRole::Secondary)
        };
        let g = util::galley(ui, title, font, color);
        ui.painter().galley(
            egui::pos2(rect.min.x + 30.0, rect.center().y - g.size().y / 2.0),
            g,
            color,
        );
    }
    ui.add_space(2.0);
    resp
}

fn emath_rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}
