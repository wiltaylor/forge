//! Non-plain-text block kinds: code (live-highlighted editor), tables
//! (label grid ↔ cell-edit grid), admonitions, dividers, ratio-weighted
//! columns with drag grips, and consumer-registered custom blocks.

use super::inline::{inline_job, text_style, InlineStyle};
use super::{render_block, text, Action, BlockEditorState, CaretHint, Ecx};
use crate::theme::{FontWeight, Severity};
use crate::widgets::specialty::code::highlight_job;
use egui::{
    Align, CornerRadius, Frame, Key, Layout, Margin, Modifiers, Pos2, Rect, Sense, Stroke, Ui, Vec2,
};
use forge_blocks::{
    set_column_ratios, table_insert_col, table_insert_row, table_remove_col, table_remove_row,
    Address, BlockKind, Document, Tone,
};

fn severity(tone: Tone) -> Severity {
    match tone {
        Tone::Info => Severity::Info,
        Tone::Success => Severity::Success,
        Tone::Warning => Severity::Warning,
        Tone::Danger => Severity::Danger,
    }
}

/* ---------------- divider ---------------- */

pub(super) fn divider(ui: &mut Ui, ecx: &mut Ecx, st: &mut BlockEditorState, addr: Address) {
    let selected = st.focus == Some(addr) && !st.editing;
    let sense = if ecx.read_only {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), sense);
    let color = if selected {
        ecx.t.accent.base
    } else {
        ecx.t.border.default
    };
    ui.painter().line_segment(
        [
            Pos2::new(rect.min.x, rect.center().y),
            Pos2::new(rect.max.x, rect.center().y),
        ],
        Stroke::new(1.0, color),
    );
    if resp.clicked() {
        ecx.actions.push(Action::Select(addr));
    }
}

/* ---------------- code ---------------- */

pub(super) fn code_block(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let t = ecx.t;
    let body_id = id.with("code-body");

    // Key interception for the focused body (Escape exits to selection,
    // Alt+arrows move the block); Enter stays native — it's a newline.
    if !ecx.read_only && ui.ctx().memory(|m| m.has_focus(body_id)) {
        let (esc, alt_up, alt_down) = ui.ctx().input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.consume_key(Modifiers::ALT, Key::ArrowUp),
                i.consume_key(Modifiers::ALT, Key::ArrowDown),
            )
        });
        if esc {
            st.editing = false;
            ui.ctx().memory_mut(|m| m.surrender_focus(body_id));
        }
        if alt_up {
            ecx.actions.push(Action::MoveBlock { addr, dir: -1 });
        }
        if alt_down {
            ecx.actions.push(Action::MoveBlock { addr, dir: 1 });
        }
    }

    let Some(BlockKind::Code { lang, code }) = doc.block_mut(addr).map(|b| &mut b.kind) else {
        return;
    };
    let mut copy_requested = false;

    Frame::new()
        .fill(t.bg[1])
        .stroke(Stroke::new(1.0, t.border.subtle))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 16.0);
            ui.horizontal(|ui| {
                if ecx.read_only {
                    ui.label(
                        egui::RichText::new(if lang.is_empty() {
                            "text"
                        } else {
                            lang.as_str()
                        })
                        .font(t.mono(t.type_scale.xs))
                        .color(t.fg[2]),
                    );
                } else {
                    let resp = ui.add(
                        egui::TextEdit::singleline(lang)
                            .id(id.with("code-lang"))
                            .frame(egui::Frame::NONE)
                            .font(t.mono(t.type_scale.xs))
                            .text_color(t.fg[2])
                            .hint_text(egui::RichText::new("lang").color(t.fg[3]))
                            .desired_width(90.0),
                    );
                    if resp.changed() {
                        st.changed = true;
                    }
                    if resp.has_focus() {
                        st.focus = Some(addr);
                        st.editing = true;
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (rect, resp) = ui.allocate_exact_size(
                        Vec2::new(40.0, 18.0),
                        if ecx.read_only {
                            Sense::hover()
                        } else {
                            Sense::click()
                        },
                    );
                    let color = if resp.hovered() { t.fg[0] } else { t.fg[2] };
                    let g = ui.painter().layout_no_wrap(
                        "copy".to_owned(),
                        t.mono(t.type_scale.xs),
                        color,
                    );
                    ui.painter().galley(
                        Pos2::new(rect.max.x - g.size().x, rect.center().y - g.size().y / 2.0),
                        g,
                        color,
                    );
                    if resp.clicked() {
                        copy_requested = true;
                    }
                });
            });
            ui.add_space(2.0);

            let lang_for_layout = lang.clone();
            if ecx.read_only {
                let job = highlight_job(ui, t, code, &lang_for_layout, t.type_scale.sm);
                ui.add(egui::Label::new(job));
                return;
            }
            let mut layouter = |ui: &Ui,
                                buf: &dyn egui::TextBuffer,
                                wrap_width: f32|
             -> std::sync::Arc<egui::Galley> {
                let mut job = highlight_job(ui, t, buf.as_str(), &lang_for_layout, t.type_scale.sm);
                job.wrap.max_width = wrap_width;
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let out = egui::TextEdit::multiline(code)
                .id(body_id)
                .frame(egui::Frame::NONE)
                .font(t.mono(t.type_scale.sm))
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .lock_focus(true)
                .layouter(&mut layouter)
                .show(ui);
            if out.response.changed() {
                st.changed = true;
            }
            if out.response.has_focus() {
                st.focus = Some(addr);
                st.editing = true;
                ui.ctx().memory_mut(|m| {
                    m.set_focus_lock_filter(
                        body_id,
                        egui::EventFilter {
                            tab: true,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                        },
                    );
                });
            }
        });

    if copy_requested {
        let text = doc
            .block(addr)
            .and_then(|b| match &b.kind {
                BlockKind::Code { code, .. } => Some(code.clone()),
                _ => None,
            })
            .unwrap_or_default();
        ui.ctx().copy_text(text);
    }
    if st.pending_code == Some(addr) {
        ui.ctx().memory_mut(|m| m.request_focus(body_id));
        st.pending_code = None;
    }
}

/* ---------------- table ---------------- */

fn table_cell_id(id: egui::Id, r: usize, c: usize) -> egui::Id {
    id.with(("cell", r, c))
}

pub(super) fn table_block(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let editing_here = st.focus == Some(addr) && st.editing && !ecx.read_only;
    if editing_here {
        table_edit(ui, ecx, st, doc, addr, id);
    } else {
        table_static(ui, ecx, doc, addr);
    }
}

/// Unfocused table: a light grid of inline-markdown labels, header row on
/// `bg[2]` — click anywhere to enter cell editing.
fn table_static(ui: &mut Ui, ecx: &mut Ecx, doc: &Document, addr: Address) {
    let Some(BlockKind::Table { header, rows }) = doc.block(addr).map(|b| &b.kind) else {
        return;
    };
    let t = ecx.t;
    let ncols = header.len().max(1);
    let avail = ui.available_width();
    let col_w = (avail - 2.0) / ncols as f32;
    let row_h = t.control.sm;
    let head_style = InlineStyle {
        size: t.type_scale.sm,
        weight: FontWeight::Medium,
        color: t.fg[0],
        italics: false,
    };
    let cell_style = InlineStyle {
        size: t.type_scale.sm,
        weight: FontWeight::Regular,
        color: t.fg[1],
        italics: false,
    };

    let total_h = row_h * (rows.len() + 1) as f32;
    let sense = if ecx.read_only {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail, total_h), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        // Header band.
        let head_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), row_h));
        painter.rect_filled(head_rect, CornerRadius::same(t.radius.sm as u8), t.bg[2]);
        let paint_row = |r: usize, cells: &[String], style: InlineStyle| {
            let y = rect.min.y + r as f32 * row_h;
            for (c, cell) in cells.iter().enumerate().take(ncols) {
                let cell_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + c as f32 * col_w, y),
                    Vec2::new(col_w, row_h),
                );
                let job = inline_job(ui, t, cell, style, f32::INFINITY);
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let pos = Pos2::new(
                    cell_rect.min.x + 8.0,
                    cell_rect.center().y - galley.size().y / 2.0,
                );
                ui.painter()
                    .with_clip_rect(cell_rect.shrink2(Vec2::new(4.0, 0.0)))
                    .galley(pos, galley, style.color);
            }
        };
        paint_row(0, header, head_style);
        for (r, row) in rows.iter().enumerate() {
            paint_row(r + 1, row, cell_style);
        }
        // Row separators.
        for r in 1..=rows.len() {
            let y = rect.min.y + r as f32 * row_h;
            ui.painter().line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(1.0, t.border.subtle),
            );
        }
    }
    if resp.clicked() {
        ecx.actions.push(Action::Focus(addr, CaretHint::End));
    }
}

/// Focused table: one small `TextEdit` per cell, Tab/Shift+Tab cell nav,
/// Enter moves down (appending a row from the last one), plus a toolbar of
/// row/column ops relative to the focused cell.
fn table_edit(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let t = ecx.t;
    let (ncols, nrows) = match doc.block(addr).map(|b| &b.kind) {
        Some(BlockKind::Table { header, rows }) => (header.len().max(1), rows.len()),
        _ => return,
    };

    // Cell navigation keys, judged against last frame's focused cell.
    if let Some((r, c)) = st.cell {
        let (tab, shift_tab, enter, esc) = ui.ctx().input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Tab),
                i.consume_key(Modifiers::SHIFT, Key::Tab),
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        let request = |st: &mut BlockEditorState, r: usize, c: usize| {
            st.pending_cell = Some((r, c));
        };
        if tab {
            let flat = r * ncols + c + 1;
            if flat < (nrows + 1) * ncols {
                request(st, flat / ncols, flat % ncols);
            }
        } else if shift_tab {
            let flat = (r * ncols + c).saturating_sub(1);
            request(st, flat / ncols, flat % ncols);
        } else if enter {
            if r < nrows {
                request(st, r + 1, c);
            } else {
                ecx.actions.push(Action::AppendTableRow { addr, col: c });
            }
        } else if esc {
            st.editing = false;
            st.cell = None;
            ui.ctx()
                .memory_mut(|m| m.surrender_focus(table_cell_id(id, r, c)));
        }
    }

    let mut focused_cell: Option<(usize, usize)> = None;
    let mut intent: Option<u8> = None; // 0 +row 1 -row 2 +col 3 -col

    {
        let Some(BlockKind::Table { header, rows }) = doc.block_mut(addr).map(|b| &mut b.kind)
        else {
            return;
        };
        Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                let col_w = ((ui.available_width() - 8.0) / ncols as f32 - 8.0).max(40.0);
                let mut cell_edit = |ui: &mut Ui,
                                     st: &mut BlockEditorState,
                                     r: usize,
                                     c: usize,
                                     text: &mut String,
                                     header: bool| {
                    let cell_id = table_cell_id(id, r, c);
                    let weight = if header {
                        FontWeight::Medium
                    } else {
                        FontWeight::Regular
                    };
                    let resp = ui.add(
                        egui::TextEdit::singleline(text)
                            .id(cell_id)
                            .frame(egui::Frame::NONE)
                            .font(t.font(ui.ctx(), weight, t.type_scale.sm))
                            .text_color(if header { t.fg[0] } else { t.fg[1] })
                            .desired_width(col_w)
                            .lock_focus(true),
                    );
                    if resp.changed() {
                        st.changed = true;
                    }
                    if resp.has_focus() {
                        focused_cell = Some((r, c));
                        ui.ctx().memory_mut(|m| {
                            m.set_focus_lock_filter(
                                cell_id,
                                egui::EventFilter {
                                    tab: true,
                                    horizontal_arrows: true,
                                    vertical_arrows: false,
                                    escape: true,
                                },
                            );
                        });
                    }
                    if st.pending_cell == Some((r, c)) {
                        ui.ctx().memory_mut(|m| m.request_focus(cell_id));
                        st.pending_cell = None;
                    }
                };
                egui::Grid::new(id.with("table-grid"))
                    .spacing(Vec2::new(8.0, 4.0))
                    .min_col_width(col_w)
                    .show(ui, |ui| {
                        for (c, cell) in header.iter_mut().enumerate() {
                            cell_edit(ui, st, 0, c, cell, true);
                        }
                        ui.end_row();
                        for (r, row) in rows.iter_mut().enumerate() {
                            for (c, cell) in row.iter_mut().enumerate().take(ncols) {
                                cell_edit(ui, st, r + 1, c, cell, false);
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mut tool = |label: &str, code: u8| {
                        if crate::widgets::Button::new(label)
                            .small(true)
                            .variant(crate::widgets::Variant::Ghost)
                            .show(ui)
                            .clicked()
                        {
                            intent = Some(code);
                        }
                    };
                    tool("+ Row", 0);
                    tool("− Row", 1);
                    tool("+ Col", 2);
                    tool("− Col", 3);
                });
            });
    }

    if let Some(cell) = focused_cell {
        st.cell = Some(cell);
    } else if st.pending_cell.is_none() {
        st.cell = None;
    }

    if let Some(op) = intent {
        let (r, c) = st.cell.unwrap_or((0, 0));
        let data_row = r.saturating_sub(1);
        let done = match op {
            0 => table_insert_row(doc, addr, if r == 0 { 0 } else { r }),
            1 => table_remove_row(doc, addr, data_row.min(nrows.saturating_sub(1))),
            2 => table_insert_col(doc, addr, c + 1),
            3 => table_remove_col(doc, addr, c),
            _ => false,
        };
        if done {
            st.changed = true;
        }
    }
}

/* ---------------- admonition ---------------- */

pub(super) fn admonition(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let Some(BlockKind::Admonition { tone, title, .. }) = doc.block(addr).map(|b| &b.kind) else {
        return;
    };
    let tone = *tone;
    let mut title_buf = title.clone();
    let t = ecx.t;
    let triple = *t.severity(severity(tone));
    let focused = st.focus == Some(addr) && !ecx.read_only;
    let mut cycle_tone = false;
    let mut title_changed = false;

    const BAR: f32 = 3.0;
    let inner = Frame::new()
        .fill(triple.bg)
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin {
            left: (BAR + 11.0) as i8,
            right: 12,
            top: 8,
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Tone badge — click cycles info → success → warning → danger.
                let label = match tone {
                    Tone::Info => "info",
                    Tone::Success => "success",
                    Tone::Warning => "warning",
                    Tone::Danger => "danger",
                };
                let g = ui.painter().layout_no_wrap(
                    label.to_owned(),
                    t.mono(t.type_scale.xs),
                    triple.base,
                );
                let (rect, resp) = ui.allocate_exact_size(
                    g.size() + Vec2::new(10.0, 6.0),
                    if ecx.read_only {
                        Sense::hover()
                    } else {
                        Sense::click()
                    },
                );
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(t.radius.sm as u8),
                    Stroke::new(1.0, triple.base),
                    egui::StrokeKind::Inside,
                );
                ui.painter().galley(
                    Pos2::new(rect.min.x + 5.0, rect.center().y - g.size().y / 2.0),
                    g,
                    triple.base,
                );
                if resp.clicked() {
                    cycle_tone = true;
                }
                if !ecx.read_only && focused {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut title_buf)
                            .id(id.with("adm-title"))
                            .frame(egui::Frame::NONE)
                            .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                            .text_color(t.fg[0])
                            .hint_text(egui::RichText::new("Title").color(t.fg[3]))
                            .desired_width(ui.available_width()),
                    );
                    title_changed = resp.changed();
                    if resp.has_focus() {
                        // Keep the block in editing mode while the title is
                        // typed so the selection ring doesn't flash on.
                        st.editing = true;
                    }
                } else {
                    let shown = if title_buf.is_empty() {
                        "Callout"
                    } else {
                        title_buf.as_str()
                    };
                    let resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new(shown)
                                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base))
                                .color(t.fg[0]),
                        )
                        .sense(if ecx.read_only {
                            Sense::hover()
                        } else {
                            Sense::click()
                        }),
                    );
                    if resp.clicked() {
                        ecx.actions.push(Action::Focus(addr, CaretHint::End));
                    }
                }
            });
            ui.add_space(2.0);
            let style = text_style(t, &BlockKind::Paragraph { md: String::new() });
            text::text_body(ui, ecx, st, doc, addr, id, style);
        });

    // Solid tone bar hugging the left edge (Alert visual parity).
    let rect = inner.response.rect;
    ui.painter().rect_filled(
        Rect::from_min_max(rect.min, Pos2::new(rect.min.x + BAR, rect.max.y)),
        CornerRadius {
            nw: BAR as u8,
            sw: BAR as u8,
            ne: 0,
            se: 0,
        },
        triple.base,
    );

    if cycle_tone || title_changed {
        if let Some(BlockKind::Admonition { tone, title, .. }) =
            doc.block_mut(addr).map(|b| &mut b.kind)
        {
            if cycle_tone {
                *tone = match *tone {
                    Tone::Info => Tone::Success,
                    Tone::Success => Tone::Warning,
                    Tone::Warning => Tone::Danger,
                    Tone::Danger => Tone::Info,
                };
            }
            if title_changed {
                *title = title_buf;
            }
            st.changed = true;
        }
    }
}

/* ---------------- custom ---------------- */

pub(super) fn custom_block(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let t = ecx.t;
    let focused = st.focus == Some(addr);
    let Some(BlockKind::Custom { kind, data }) = doc.block_mut(addr).map(|b| &mut b.kind) else {
        return;
    };
    let registered = st.custom.iter_mut().find(|c| c.kind() == kind.as_str());
    let rect = match registered {
        Some(custom) => {
            let mut data_changed = false;
            let inner = ui.scope(|ui| {
                if ecx.read_only {
                    ui.add_enabled_ui(false, |ui| {
                        let _ = custom.show(ui, data, false, t);
                    });
                } else {
                    data_changed = custom.show(ui, data, focused, t);
                }
            });
            if data_changed {
                st.changed = true;
            }
            inner.response.rect
        }
        None => {
            let inner = Frame::new().inner_margin(Margin::same(10)).show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("Custom block: {kind}"))
                        .font(t.mono(t.type_scale.sm))
                        .color(t.fg[2]),
                );
            });
            let rect = inner.response.rect;
            dashed_rect(ui, rect, Stroke::new(1.0, t.border.strong));
            rect
        }
    };
    if !ecx.read_only {
        let resp = ui.interact(rect, id.with("custom-select"), Sense::click());
        if resp.clicked() {
            ecx.actions.push(Action::Select(addr));
        }
    }
}

fn dashed_rect(ui: &Ui, rect: Rect, stroke: Stroke) {
    let p = ui.painter();
    let (d, g) = (5.0, 4.0);
    p.add(egui::Shape::dashed_line(
        &[rect.left_top(), rect.right_top()],
        stroke,
        d,
        g,
    ));
    p.add(egui::Shape::dashed_line(
        &[rect.right_top(), rect.right_bottom()],
        stroke,
        d,
        g,
    ));
    p.add(egui::Shape::dashed_line(
        &[rect.right_bottom(), rect.left_bottom()],
        stroke,
        d,
        g,
    ));
    p.add(egui::Shape::dashed_line(
        &[rect.left_bottom(), rect.left_top()],
        stroke,
        d,
        g,
    ));
}

/* ---------------- columns ---------------- */

/// A `Columns` root block: ratio-weighted cells side by side with draggable
/// grips between them; each cell renders its own block list recursively.
pub(super) fn columns_block(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    root: usize,
) {
    let (block_id, ratios, counts) = match doc.blocks.get(root) {
        Some(b) => match &b.kind {
            BlockKind::Columns { columns } => (
                b.id.clone(),
                columns.iter().map(|c| c.ratio).collect::<Vec<f32>>(),
                columns
                    .iter()
                    .map(|c| c.blocks.len())
                    .collect::<Vec<usize>>(),
            ),
            _ => return,
        },
        None => return,
    };
    let ncols = ratios.len();
    if ncols == 0 {
        return;
    }
    const GRIP: f32 = 10.0;
    let avail = ui.available_width();
    let total = (avail - GRIP * (ncols - 1) as f32).max(60.0);
    let sum: f32 = ratios.iter().sum::<f32>().max(0.001);

    let mut grip_x: Vec<f32> = Vec::new();
    let row = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (c, ratio) in ratios.iter().enumerate() {
            let w = total * ratio / sum;
            ui.allocate_ui_with_layout(Vec2::new(w, 10.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(w);
                ui.set_max_width(w);
                ui.spacing_mut().item_spacing.y = ecx.t.space.x(1.5);
                for idx in 0..counts[c] {
                    render_block(ui, ecx, st, doc, Address::Cell { root, col: c, idx });
                }
            });
            if c + 1 < ncols {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(GRIP, 10.0), Sense::hover());
                grip_x.push(rect.center().x);
            }
        }
    });

    if ecx.read_only {
        return;
    }
    let rect = row.response.rect;
    let mut next = ratios.clone();
    let mut dragged = false;
    for (k, x) in grip_x.iter().enumerate() {
        let grip_rect = Rect::from_center_size(
            Pos2::new(*x, rect.center().y),
            Vec2::new(GRIP, rect.height()),
        );
        let resp = ui.interact(
            grip_rect,
            egui::Id::new(("forge-col-grip", block_id.as_str(), k)),
            Sense::drag(),
        );
        let active = resp.hovered() || resp.dragged();
        if active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        ui.painter().line_segment(
            [
                Pos2::new(*x, rect.min.y + 2.0),
                Pos2::new(*x, rect.max.y - 2.0),
            ],
            Stroke::new(
                1.0,
                if active {
                    ecx.t.accent.base
                } else {
                    ecx.t.border.subtle
                },
            ),
        );
        if resp.dragged() {
            let delta = resp.drag_delta().x / total;
            next[k] += delta;
            next[k + 1] -= delta;
            dragged = true;
        }
    }
    if dragged && next.iter().all(|r| *r > 0.02) && set_column_ratios(doc, root, &next) {
        st.changed = true;
    }
}
