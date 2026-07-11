//! The Forge app frame: topbar, grouped sidebar nav, status bar, content.
//! Mirrors `forge_tui::runtime::AppShell` and the web `AppShell`.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{
    FontWeight, Theme, SIDEBAR_RAIL, SIDEBAR_WIDTH, STATUSBAR_HEIGHT, TOPBAR_HEIGHT,
};
use crate::widgets::primitives::Glyph;
use egui::{CornerRadius, Sense, Stroke, Ui, Vec2};

pub struct NavItem<'a> {
    pub label: &'a str,
    pub icon: Option<Glyph>,
    pub count: Option<u32>,
}

impl<'a> NavItem<'a> {
    pub fn new(label: &'a str) -> NavItem<'a> {
        NavItem {
            label,
            icon: None,
            count: None,
        }
    }

    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
}

pub struct NavSection<'a> {
    pub title: Option<&'a str>,
    pub items: Vec<NavItem<'a>>,
}

impl<'a> NavSection<'a> {
    pub fn new(title: Option<&'a str>, labels: &[&'a str]) -> NavSection<'a> {
        NavSection {
            title,
            items: labels.iter().map(|l| NavItem::new(l)).collect(),
        }
    }

    pub fn items(title: Option<&'a str>, items: Vec<NavItem<'a>>) -> NavSection<'a> {
        NavSection { title, items }
    }
}

/// Persistent shell state, owned by the app.
#[derive(Clone, Debug, Default)]
pub struct ShellState {
    /// Flattened selected nav index across all sections.
    pub selected: usize,
    /// Sidebar collapsed to the slim icon rail. `None` = expanded.
    pub collapsed: bool,
}

type SlotFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

pub struct Shell<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
    sections: &'a [NavSection<'a>],
    topbar: Option<&'a str>,
    topbar_right: Option<SlotFn<'a>>,
    status: Option<&'a str>,
    status_right: Option<&'a str>,
}

impl<'a> Shell<'a> {
    pub fn new(title: &'a str, sections: &'a [NavSection<'a>]) -> Shell<'a> {
        Shell {
            title,
            subtitle: None,
            sections,
            topbar: None,
            topbar_right: None,
            status: None,
            status_right: None,
        }
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Topbar title (usually the active page name).
    pub fn topbar(mut self, title: &'a str) -> Self {
        self.topbar = Some(title);
        self
    }

    /// Custom widgets on the right end of the topbar.
    pub fn topbar_right(mut self, f: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.topbar_right = Some(Box::new(f));
        self
    }

    pub fn status(mut self, status: &'a str) -> Self {
        self.status = Some(status);
        self
    }

    pub fn status_right(mut self, status: &'a str) -> Self {
        self.status_right = Some(status);
        self
    }

    /// Lay out the shell chrome and render the page into the content region.
    /// Returns `Changed` when the nav selection moved this frame.
    pub fn show(
        self,
        ui: &mut Ui,
        state: &mut ShellState,
        content: impl FnOnce(&mut Ui),
    ) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let mut outcome = Outcome::Ignored;

        // Ctrl+B toggles the sidebar rail, unless something consumed it.
        if ui.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::CTRL,
                egui::Key::B,
            ) || i.consume_key(egui::Modifiers::CTRL, egui::Key::B)
        }) {
            state.collapsed = !state.collapsed;
        }

        // Topbar.
        egui::Panel::top("forge-topbar")
            .exact_size(TOPBAR_HEIGHT)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(t.bg[1])
                    .inner_margin(egui::Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1.0, t.border.subtle),
                );
                ui.horizontal_centered(|ui| {
                    if let Some(title) = self.topbar {
                        ui.label(
                            egui::RichText::new(title)
                                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.md))
                                .color(t.fg[0]),
                        );
                    }
                    if let Some(right) = self.topbar_right {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
                    }
                });
            });

        // Status bar.
        egui::Panel::bottom("forge-status")
            .exact_size(STATUSBAR_HEIGHT)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(t.bg[1])
                    .inner_margin(egui::Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    Stroke::new(1.0, t.border.subtle),
                );
                ui.horizontal_centered(|ui| {
                    if let Some(status) = self.status {
                        ui.label(
                            egui::RichText::new(status)
                                .size(t.type_scale.sm)
                                .color(t.fg[2]),
                        );
                    }
                    if let Some(right) = self.status_right {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(right)
                                    .size(t.type_scale.sm)
                                    .color(t.fg[2]),
                            );
                        });
                    }
                });
            });

        // Sidebar.
        let width = if state.collapsed {
            SIDEBAR_RAIL
        } else {
            SIDEBAR_WIDTH
        };
        egui::Panel::left("forge-sidebar")
            .exact_size(width)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(t.bg[1]))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().line_segment(
                    [rect.right_top(), rect.right_bottom()],
                    Stroke::new(1.0, t.border.subtle),
                );
                ui.add_space(14.0);
                if !state.collapsed {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(self.title)
                                    .font(t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.md))
                                    .color(t.fg[0]),
                            );
                            if let Some(subtitle) = self.subtitle {
                                ui.label(
                                    egui::RichText::new(subtitle)
                                        .size(t.type_scale.xs)
                                        .color(t.fg[2]),
                                );
                            }
                        });
                    });
                    ui.add_space(10.0);
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut flat = 0usize;
                    for section in self.sections {
                        if let (Some(title), false) = (section.title, state.collapsed) {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                crate::widgets::Eyebrow::new(title).show(ui);
                            });
                            ui.add_space(2.0);
                        } else if section.title.is_some() {
                            ui.add_space(8.0);
                        }
                        for item in &section.items {
                            let index = flat;
                            flat += 1;
                            if nav_row(ui, &t, item, index == state.selected, state.collapsed)
                                && state.selected != index
                            {
                                state.selected = index;
                                outcome = Outcome::Changed;
                            }
                        }
                    }
                });
            });

        // Content region fills the remainder.
        let response = egui::Frame::new()
            .fill(t.bg[0])
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                content(ui);
            })
            .response;

        ForgeResponse::new(response, outcome)
    }
}

/// One nav row: accent tint + left bar when selected, hover fill otherwise.
/// Returns true on click.
fn nav_row(ui: &mut Ui, t: &Theme, item: &NavItem, selected: bool, collapsed: bool) -> bool {
    let height = 30.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            true,
            selected,
            item.label,
        )
    });

    if ui.is_rect_visible(rect) {
        let inner = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + 8.0, rect.min.y + 1.0),
            egui::pos2(rect.max.x - 8.0, rect.max.y - 1.0),
        );
        if selected {
            ui.painter()
                .rect_filled(inner, CornerRadius::same(t.radius.md as u8), t.accent.bg);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(inner.min, egui::pos2(inner.min.x + 2.0, inner.max.y)),
                0.0,
                t.accent.base,
            );
        } else if response.hovered() {
            ui.painter()
                .rect_filled(inner, CornerRadius::same(t.radius.md as u8), t.bg[2]);
        }

        let text_color = if selected { t.accent.fg } else { t.fg[1] };
        let mut x = inner.min.x + 10.0;
        if let Some(icon) = item.icon {
            let g = ui.painter().layout_no_wrap(
                icon.as_str().to_owned(),
                egui::FontId::proportional(t.type_scale.base),
                text_color,
            );
            ui.painter().galley(
                egui::pos2(x, rect.center().y - g.size().y / 2.0),
                g,
                text_color,
            );
            x += 20.0;
        }
        if !collapsed {
            let g = ui.painter().layout_no_wrap(
                item.label.to_owned(),
                egui::FontId::proportional(t.type_scale.base),
                text_color,
            );
            ui.painter().galley(
                egui::pos2(x, rect.center().y - g.size().y / 2.0),
                g,
                text_color,
            );
            if let Some(count) = item.count {
                let cg = ui.painter().layout_no_wrap(
                    count.to_string(),
                    egui::FontId::proportional(t.type_scale.xs),
                    t.fg[2],
                );
                let cx = inner.max.x - cg.size().x - 10.0;
                ui.painter().galley(
                    egui::pos2(cx, rect.center().y - cg.size().y / 2.0),
                    cg,
                    t.fg[2],
                );
            }
        }
    }
    response.clicked()
}
