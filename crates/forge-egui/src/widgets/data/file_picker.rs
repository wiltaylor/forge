//! Filesystem browser over `std::fs`. IO happens only on navigation —
//! entries are cached in the state and refreshed when the directory
//! changes, never per frame.
//!
//! Double-click (or Enter on the focused row) descends into directories and
//! submits files (`Outcome::Submitted`, path in `state.selected`); the `..`
//! row ascends; breadcrumb segments jump straight to an ancestor.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Surface, TextRole, Theme};
use crate::widgets::primitives::Glyph;
use crate::widgets::util;
use egui::{CornerRadius, Key, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    is_dir: bool,
}

/// Directory cursor + cached listing. Create with a start directory; call
/// [`FilePickerState::refresh`] if the filesystem changed underneath you.
#[derive(Clone, Debug)]
pub struct FilePickerState {
    /// Current directory (navigate with the widget or [`Self::set_dir`]).
    pub dir: PathBuf,
    /// The file chosen by double-click/Enter, if any.
    pub selected: Option<PathBuf>,
    /// Show dotfiles.
    pub show_hidden: bool,
    entries: Vec<Entry>,
    error: Option<String>,
    /// Row highlighted by a single click (relative index into `entries`).
    cursor: Option<usize>,
}

impl FilePickerState {
    pub fn new(dir: impl Into<PathBuf>) -> FilePickerState {
        let mut s = FilePickerState {
            dir: dir.into(),
            selected: None,
            show_hidden: false,
            entries: Vec::new(),
            error: None,
            cursor: None,
        };
        s.refresh();
        s
    }

    /// Jump to a directory (re-reads the listing).
    pub fn set_dir(&mut self, dir: impl Into<PathBuf>) {
        self.dir = dir.into();
        self.refresh();
    }

    /// Re-read the current directory. The only place IO happens.
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;
        self.cursor = None;
        match std::fs::read_dir(&self.dir) {
            Ok(read) => {
                for entry in read.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    self.entries.push(Entry { name, is_dir });
                }
                // Dirs first, then case-insensitive by name.
                self.entries.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn descend(&mut self, name: &str) {
        self.dir.push(name);
        self.refresh();
    }

    fn ascend(&mut self) -> bool {
        if self.dir.pop() {
            self.refresh();
            true
        } else {
            false
        }
    }
}

/// The picker view: breadcrumb header + hidden-files chip + listing.
pub struct FilePicker<'a> {
    state: &'a mut FilePickerState,
    height: f32,
}

impl<'a> FilePicker<'a> {
    pub fn new(state: &'a mut FilePickerState) -> FilePicker<'a> {
        FilePicker {
            state,
            height: 260.0,
        }
    }

    /// Listing viewport height (default 260).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self { state, height } = self;
        let mut outcome = Outcome::Ignored;

        // ---- Breadcrumb header + hidden toggle.
        let mut jump: Option<PathBuf> = None;
        ui.horizontal(|ui| {
            let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
            let comps: Vec<PathBuf> = state
                .dir
                .ancestors()
                .map(Path::to_path_buf)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            for (i, ancestor) in comps.iter().enumerate() {
                let name = match ancestor.file_name() {
                    Some(n) => n.to_string_lossy().into_owned(),
                    None => ancestor.display().to_string(), // "/" or drive root
                };
                let last = i + 1 == comps.len();
                let color = if last {
                    t.text(TextRole::Primary)
                } else {
                    t.text(TextRole::Tertiary)
                };
                let g = util::galley(ui, &name, font.clone(), color);
                let (rect, resp) = ui.allocate_exact_size(
                    g.size() + Vec2::new(8.0, 8.0),
                    if last { Sense::hover() } else { Sense::click() },
                );
                resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, !last, &name));
                if ui.is_rect_visible(rect) {
                    if resp.hovered() && !last {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(t.radius.sm as u8),
                            t.surface(Surface::Hover),
                        );
                    }
                    let color = if resp.hovered() && !last {
                        t.text(TextRole::Primary)
                    } else {
                        color
                    };
                    let g = util::galley(ui, &name, font.clone(), color);
                    ui.painter()
                        .galley(rect.center() - g.size() / 2.0, g, color);
                }
                if resp.clicked() {
                    jump = Some(ancestor.clone());
                }
                if !last {
                    let g = util::galley(ui, "/", font.clone(), t.text(TextRole::Disabled));
                    let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                    ui.painter().galley(r.min, g, t.text(TextRole::Disabled));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let chip = hidden_chip(ui, &t, state.show_hidden);
                if chip.clicked() {
                    state.show_hidden = !state.show_hidden;
                    state.refresh();
                    outcome = outcome.merge(Outcome::Changed);
                }
            });
        });
        if let Some(dir) = jump {
            if dir != state.dir {
                state.set_dir(dir);
                outcome = outcome.merge(Outcome::Changed);
            }
        }
        ui.add_space(t.space.x(1.5));

        // ---- Listing.
        let mut nav: Option<Nav> = None;
        let frame = egui::Frame::new()
            .fill(t.surface(Surface::Card))
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(egui::Margin::same(4));
        let inner = frame.show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(ui.id().with("file-picker"))
                .max_height(height)
                .min_scrolled_height(height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if let Some(err) = &state.error {
                        ui.label(
                            egui::RichText::new(err)
                                .font(t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm))
                                .color(t.danger.fg),
                        );
                        return;
                    }
                    let enter = ui.input(|i| i.key_pressed(Key::Enter));
                    // ".." row (unless at a filesystem root).
                    if state.dir.parent().is_some() {
                        let r = row_ui(ui, &t, "..", Glyph::Folder, true, false);
                        if r.double_clicked() || (enter && r.has_focus()) {
                            nav = Some(Nav::Up);
                        } else if r.clicked() {
                            state.cursor = None;
                        }
                    }
                    for (i, entry) in state.entries.iter().enumerate() {
                        let cursor = state.cursor == Some(i);
                        let r = row_ui(
                            ui,
                            &t,
                            &entry.name,
                            if entry.is_dir {
                                Glyph::Folder
                            } else {
                                Glyph::File
                            },
                            entry.is_dir,
                            cursor,
                        );
                        if r.double_clicked() || (enter && r.has_focus()) {
                            nav = Some(if entry.is_dir {
                                Nav::Descend(entry.name.clone())
                            } else {
                                Nav::Pick(entry.name.clone())
                            });
                        } else if r.clicked() && !cursor {
                            nav = Some(Nav::Cursor(i));
                        }
                    }
                });
        });

        match nav {
            Some(Nav::Up) => {
                if state.ascend() {
                    outcome = outcome.merge(Outcome::Changed);
                }
            }
            Some(Nav::Descend(name)) => {
                state.descend(&name);
                outcome = outcome.merge(Outcome::Changed);
            }
            Some(Nav::Pick(name)) => {
                state.selected = Some(state.dir.join(name));
                outcome = Outcome::Submitted;
            }
            Some(Nav::Cursor(i)) => {
                state.cursor = Some(i);
                outcome = outcome.merge(Outcome::Consumed);
            }
            None => {}
        }
        ForgeResponse::new(inner.response, outcome)
    }
}

enum Nav {
    Up,
    Descend(String),
    Pick(String),
    Cursor(usize),
}

fn row_ui(
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    glyph: Glyph,
    is_dir: bool,
    cursor: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), t.control.sm),
        Sense::click(),
    );
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, cursor, name));
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(t.radius.sm as u8);
        if cursor {
            ui.painter().rect_filled(rect, radius, t.accent.bg);
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, radius, t.surface(Surface::Hover));
        }
        let font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base);
        let cy = rect.center().y;
        let gcolor = if is_dir {
            t.accent.fg
        } else {
            t.text(TextRole::Disabled)
        };
        let g = util::galley(ui, glyph.as_str(), font.clone(), gcolor);
        ui.painter().galley(
            egui::pos2(rect.min.x + 8.0, cy - g.size().y / 2.0),
            g,
            gcolor,
        );
        let color = if cursor {
            t.accent.fg
        } else if is_dir {
            t.text(TextRole::Primary)
        } else {
            t.text(TextRole::Secondary)
        };
        let g = util::galley(ui, name, font, color);
        let clip = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        clip.galley(
            egui::pos2(rect.min.x + 30.0, cy - g.size().y / 2.0),
            g,
            color,
        );
    }
    resp
}

/// The "Hidden" dotfiles chip.
fn hidden_chip(ui: &mut Ui, t: &Theme, on: bool) -> Response {
    let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
    let g = util::galley(ui, "Hidden", font.clone(), t.text(TextRole::Secondary));
    let size = Vec2::new(g.size().x + 20.0, t.control.sm - 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, "Hidden"));
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same((rect.height() / 2.0) as u8);
        let (fill, color) = if on {
            (t.accent.bg, t.accent.fg)
        } else if resp.hovered() {
            (t.surface(Surface::Pressed), t.text(TextRole::Primary))
        } else {
            (t.surface(Surface::Hover), t.text(TextRole::Secondary))
        };
        ui.painter().rect_filled(rect, radius, fill);
        let g = util::galley(ui, "Hidden", font, color);
        ui.painter()
            .galley(rect.center() - g.size() / 2.0, g, color);
    }
    resp
}
