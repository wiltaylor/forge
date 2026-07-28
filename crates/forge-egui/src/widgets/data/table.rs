//! Flat, border-free sortable data table — plain egui, no `egui_extras`.
//!
//! Sorting is the *caller's* job: clicking a header cycles
//! `state.sort` (asc → desc → none) and the header shows the caret, but the
//! widget never reorders anything. Order your own row data from `state.sort`
//! before calling [`Table::show`] (see the gallery's data section).

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use egui::{
    Align, Color32, CornerRadius, Rect, Response, Sense, Ui, UiBuilder, Vec2, WidgetInfo,
    WidgetType,
};

/// How a [`Column`] takes horizontal space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColWidth {
    /// Sized to the header title (plus caret allowance).
    #[default]
    Auto,
    /// Exactly this many points.
    Fixed(f32),
    /// Splits whatever is left after fixed/auto columns (equally among all
    /// `Remainder` columns).
    Remainder,
}

/// Sort direction carried in [`TableState::sort`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

/// Column definition: title, sizing, and cell alignment.
#[derive(Clone, Copy, Debug)]
pub struct Column<'a> {
    pub(crate) title: &'a str,
    pub(crate) width: ColWidth,
    pub(crate) align: Align,
}

impl<'a> Column<'a> {
    pub fn new(title: &'a str) -> Column<'a> {
        Column {
            title,
            width: ColWidth::Auto,
            align: Align::Min,
        }
    }

    pub fn width(mut self, width: ColWidth) -> Self {
        self.width = width;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
}

/// Sort request + row selection. Plain data — persist it wherever your app
/// state lives. `sort` is what the header shows; apply it to your rows
/// yourself.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TableState {
    pub sort: Option<(usize, SortDir)>,
    pub selected: Option<usize>,
}

/// One row under construction — hand cells to it in column order.
pub struct TableRow<'u, 'c> {
    ui: &'u mut Ui,
    theme: &'u Theme,
    columns: &'c [Column<'c>],
    widths: &'u [f32],
    rect: Rect,
    index: usize,
    col: usize,
    text_color: Color32,
    first_text: Option<String>,
}

impl TableRow<'_, '_> {
    /// Index of the row being built (post-sort display order).
    pub fn index(&self) -> usize {
        self.index
    }

    fn cell_rect(&self) -> Rect {
        let x0: f32 = self.rect.min.x + self.widths[..self.col].iter().sum::<f32>();
        Rect::from_min_size(
            egui::pos2(x0, self.rect.min.y),
            Vec2::new(self.widths[self.col], self.rect.height()),
        )
    }

    /// A plain text cell in the row foreground color.
    pub fn text(&mut self, text: impl AsRef<str>) {
        self.text_colored(text, self.text_color);
    }

    /// A plain text cell in a custom color.
    pub fn text_colored(&mut self, text: impl AsRef<str>, color: Color32) {
        let text = text.as_ref();
        if self.col >= self.columns.len() {
            return;
        }
        if self.first_text.is_none() {
            self.first_text = Some(text.to_owned());
        }
        let rect = self.cell_rect();
        let font = self.theme.font(
            self.ui.ctx(),
            FontWeight::Regular,
            self.theme.type_scale.base,
        );
        let g = util::galley(self.ui, text, font, color);
        let x = match self.columns[self.col].align {
            Align::Min => rect.min.x + CELL_PAD,
            Align::Center => rect.center().x - g.size().x / 2.0,
            Align::Max => rect.max.x - CELL_PAD - g.size().x,
        };
        let clip = self
            .ui
            .painter()
            .with_clip_rect(rect.intersect(self.ui.clip_rect()));
        clip.galley(egui::pos2(x, rect.center().y - g.size().y / 2.0), g, color);
        self.col += 1;
    }

    /// An arbitrary-content cell (badges, buttons, …).
    pub fn cell(&mut self, add: impl FnOnce(&mut Ui)) {
        if self.col >= self.columns.len() {
            return;
        }
        let rect = self.cell_rect().shrink2(Vec2::new(CELL_PAD, 0.0));
        let layout = match self.columns[self.col].align {
            Align::Max => egui::Layout::right_to_left(Align::Center),
            _ => egui::Layout::left_to_right(Align::Center),
        };
        let mut child = self
            .ui
            .new_child(UiBuilder::new().max_rect(rect).layout(layout));
        add(&mut child);
        self.col += 1;
    }
}

const CELL_PAD: f32 = 8.0;
const CARET_ALLOWANCE: f32 = 14.0;

/// The table widget. Header sort clicks and row selection land in
/// [`TableState`]; `Changed` when either moves.
pub struct Table<'a> {
    state: &'a mut TableState,
    columns: &'a [Column<'a>],
    striped: bool,
    max_height: Option<f32>,
}

impl<'a> Table<'a> {
    pub fn new(state: &'a mut TableState, columns: &'a [Column<'a>]) -> Table<'a> {
        Table {
            state,
            columns,
            striped: false,
            max_height: None,
        }
    }

    /// Tint every other row for scanability.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Rows scroll inside this height (default: unconstrained).
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    fn widths(&self, ui: &Ui, t: &Theme, total: f32) -> Vec<f32> {
        let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        let mut widths: Vec<f32> = self
            .columns
            .iter()
            .map(|c| match c.width {
                ColWidth::Fixed(w) => w,
                ColWidth::Auto => {
                    let g = util::galley(ui, c.title, font.clone(), Color32::WHITE);
                    g.size().x + CELL_PAD * 2.0 + CARET_ALLOWANCE
                }
                ColWidth::Remainder => 0.0,
            })
            .collect();
        let used: f32 = widths.iter().sum();
        let remainders = self
            .columns
            .iter()
            .filter(|c| c.width == ColWidth::Remainder)
            .count();
        if remainders > 0 {
            let share = ((total - used) / remainders as f32).max(0.0);
            for (w, c) in widths.iter_mut().zip(self.columns) {
                if c.width == ColWidth::Remainder {
                    *w = share;
                }
            }
        }
        widths
    }

    pub fn show(
        self,
        ui: &mut Ui,
        row_count: usize,
        mut row_ui: impl FnMut(&mut TableRow<'_, '_>),
    ) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let total = ui.available_width();
        let widths = self.widths(ui, &t, total);
        let Self {
            state,
            columns,
            striped,
            max_height,
        } = self;
        let table_w: f32 = widths.iter().sum::<f32>().min(total);
        let row_h = t.control.sm;
        let mut outcome = Outcome::Ignored;

        ui.spacing_mut().item_spacing.y = 0.0;

        // ---- Header row (sticky by construction: outside the ScrollArea).
        let (head_rect, head_resp) =
            ui.allocate_exact_size(Vec2::new(table_w, row_h), Sense::hover());
        let mut union: Response = head_resp;
        if ui.is_rect_visible(head_rect) {
            ui.painter()
                .rect_filled(head_rect, CornerRadius::same(t.radius.sm as u8), t.bg[1]);
            ui.painter().hline(
                head_rect.x_range(),
                head_rect.max.y - 0.5,
                egui::Stroke::new(1.0, t.border.default),
            );
        }
        let font_head = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        let mut x = head_rect.min.x;
        for (i, (col, w)) in columns.iter().zip(&widths).enumerate() {
            let rect = Rect::from_min_size(egui::pos2(x, head_rect.min.y), Vec2::new(*w, row_h));
            let resp = ui.interact(rect, ui.id().with(("th", i)), Sense::click());
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, col.title));
            if resp.clicked() {
                state.sort = match state.sort {
                    Some((c, SortDir::Asc)) if c == i => Some((i, SortDir::Desc)),
                    Some((c, SortDir::Desc)) if c == i => None,
                    _ => Some((i, SortDir::Asc)),
                };
                outcome = Outcome::Changed;
            }
            if ui.is_rect_visible(rect) {
                let sorted = matches!(state.sort, Some((c, _)) if c == i);
                let color = if sorted {
                    t.accent.fg
                } else if resp.hovered() {
                    t.fg[1]
                } else {
                    t.fg[2]
                };
                let caret = match state.sort {
                    Some((c, SortDir::Asc)) if c == i => " ▴",
                    Some((c, SortDir::Desc)) if c == i => " ▾",
                    _ => "",
                };
                let g = util::galley(
                    ui,
                    format!("{}{caret}", col.title),
                    font_head.clone(),
                    color,
                );
                let tx = match col.align {
                    Align::Min => rect.min.x + CELL_PAD,
                    Align::Center => rect.center().x - g.size().x / 2.0,
                    Align::Max => rect.max.x - CELL_PAD - g.size().x,
                };
                ui.painter()
                    .galley(egui::pos2(tx, rect.center().y - g.size().y / 2.0), g, color);
            }
            union = union.union(resp);
            x += w;
        }

        // ---- Body rows, scrolling.
        let mut body = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for ri in 0..row_count {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(table_w, row_h), Sense::click());
                if resp.clicked() && state.selected != Some(ri) {
                    state.selected = Some(ri);
                    outcome = outcome.merge(Outcome::Changed);
                }
                let selected = state.selected == Some(ri);
                let text_color = if selected { t.fg[0] } else { t.fg[1] };
                if ui.is_rect_visible(rect) {
                    if striped && ri % 2 == 1 {
                        ui.painter()
                            .rect_filled(rect, 0.0, t.bg[2].gamma_multiply(0.45));
                    }
                    if resp.hovered() && !selected {
                        ui.painter().rect_filled(rect, 0.0, t.bg[2]);
                    }
                    if selected {
                        ui.painter().rect_filled(rect, 0.0, t.accent.bg);
                        ui.painter().rect_filled(
                            Rect::from_min_size(rect.min, Vec2::new(2.0, rect.height())),
                            0.0,
                            t.accent.base,
                        );
                    }
                }
                let mut row = TableRow {
                    ui,
                    theme: &t,
                    columns,
                    widths: &widths,
                    rect,
                    index: ri,
                    col: 0,
                    text_color,
                    first_text: None,
                };
                row_ui(&mut row);
                let label = row.first_text.take().unwrap_or_else(|| format!("row {ri}"));
                resp.widget_info(move || {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &label)
                });
            }
        };
        if let Some(h) = max_height {
            egui::ScrollArea::vertical()
                .id_salt(ui.id().with("table-body"))
                .max_height(h)
                .auto_shrink([false, true])
                .show(ui, body);
        } else {
            body(ui);
        }

        ForgeResponse::new(union, outcome)
    }
}
