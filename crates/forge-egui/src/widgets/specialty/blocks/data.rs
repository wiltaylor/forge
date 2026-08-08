//! Data block kinds (image, video, math, charts, diagrams, node table,
//! tree, timeline, chapter header): display renderers plus the shared
//! JSON-source editor. Charts and flowcharts reuse the kit widgets; the
//! rest paint directly. Editing follows the code-block focus contract —
//! keys are only intercepted while the JSON body owns focus.

use super::inline::{inline_job, InlineStyle};
use super::{keys, Action, BlockEditorState, CaretHint, Ecx};
use crate::theme::{series_color, FontWeight, Surface, TextRole};
use crate::widgets::charts::{BarChart, BarGroup, LineChart, LineSeries, PieChart, PieSlice};
use crate::widgets::specialty::code::highlight_job;
use crate::widgets::specialty::{FlowEdge, FlowNode, Flowchart};
use crate::widgets::Tone as WidgetTone;
use egui::{Align2, CornerRadius, Frame, Key, Margin, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use forge_blocks::{Address, BlockKind, Document, MessageKind};

/* ---------------- dispatch ---------------- */

pub(super) fn data_block(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let editing_here = st.focus == Some(addr) && st.editing && !ecx.read_only;
    if editing_here {
        json_edit(ui, ecx, st, doc, addr, id);
        return;
    }
    let Some(block) = doc.block(addr) else { return };
    let kind = block.kind.clone();
    let inner = ui.scope(|ui| render_kind(ui, ecx, st, &kind));
    if !ecx.read_only {
        let resp = ui.interact(inner.response.rect, id.with("data-select"), Sense::click());
        if resp.clicked() {
            ecx.actions.push(Action::Focus(addr, CaretHint::End));
        }
    }
}

fn render_kind(ui: &mut Ui, ecx: &mut Ecx, st: &mut BlockEditorState, kind: &BlockKind) {
    match kind {
        BlockKind::Image {
            src,
            alt,
            width,
            height,
        } => image_view(ui, ecx, st, src, alt, *width, *height),
        BlockKind::Video { src, title, .. } => {
            media_card(ui, ecx, "▶ video", title.as_deref(), src)
        }
        BlockKind::Math { tex } => math_view(ui, ecx, tex),
        BlockKind::BarChart {
            title,
            categories,
            series,
            ..
        } => {
            chart_title(ui, ecx, title.as_deref());
            let groups: Vec<BarGroup> = categories
                .iter()
                .enumerate()
                .map(|(ci, cat)| {
                    BarGroup::new(
                        cat.clone(),
                        series
                            .iter()
                            .map(|s| s.values.get(ci).copied().unwrap_or(0.0))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            let names: Vec<&str> = series.iter().map(|s| s.name.as_str()).collect();
            BarChart::new(&groups).names(&names).height(160.0).show(ui);
        }
        BlockKind::LineChart {
            title,
            categories,
            series,
            ..
        } => {
            chart_title(ui, ecx, title.as_deref());
            let ls: Vec<LineSeries> = series
                .iter()
                .map(|s| LineSeries::new(s.name.clone(), s.values.clone()))
                .collect();
            let labels: Vec<&str> = categories.iter().map(String::as_str).collect();
            LineChart::new(&ls).x_labels(&labels).height(160.0).show(ui);
        }
        BlockKind::PieChart { title, slices } => {
            chart_title(ui, ecx, title.as_deref());
            let ps: Vec<PieSlice> = slices
                .iter()
                .map(|s| PieSlice::new(s.label.clone(), s.value))
                .collect();
            PieChart::new(&ps).legend(true).height(160.0).show(ui);
        }
        BlockKind::Diagram { nodes, edges, .. } => {
            let fnodes: Vec<FlowNode> = nodes
                .iter()
                .map(|n| {
                    FlowNode::new(n.id.clone(), n.text.clone()).tone(match n.kind {
                        forge_blocks::DiagramNodeKind::Decision => WidgetTone::Warning,
                        forge_blocks::DiagramNodeKind::Terminator => WidgetTone::Info,
                        _ => WidgetTone::Neutral,
                    })
                })
                .collect();
            let fedges: Vec<FlowEdge> = edges
                .iter()
                .map(|e| {
                    let fe = FlowEdge::new(e.from.clone(), e.to.clone());
                    match &e.label {
                        Some(l) => fe.label(l.clone()),
                        None => fe,
                    }
                })
                .collect();
            Flowchart::new(&fnodes, &fedges).show(ui);
        }
        BlockKind::StateDiagram {
            states,
            transitions,
        } => {
            let fnodes: Vec<FlowNode> = states
                .iter()
                .map(|s| {
                    let name = s.name.as_deref().unwrap_or(&s.id);
                    let (label, tone) = if s.initial == Some(true) {
                        (format!("● {name}"), WidgetTone::Accent)
                    } else if s.is_final == Some(true) {
                        (format!("◉ {name}"), WidgetTone::Success)
                    } else {
                        (name.to_string(), WidgetTone::Neutral)
                    };
                    FlowNode::new(s.id.clone(), label).tone(tone)
                })
                .collect();
            let fedges: Vec<FlowEdge> = transitions
                .iter()
                .map(|tr| {
                    let fe = FlowEdge::new(tr.from.clone(), tr.to.clone());
                    let label = match (&tr.trigger, &tr.guard) {
                        (Some(t_), Some(g)) => format!("{t_} [{g}]"),
                        (Some(t_), None) => t_.clone(),
                        (None, Some(g)) => format!("[{g}]"),
                        (None, None) => String::new(),
                    };
                    if label.is_empty() {
                        fe
                    } else {
                        fe.label(label)
                    }
                })
                .collect();
            Flowchart::new(&fnodes, &fedges).show(ui);
        }
        BlockKind::SequenceDiagram {
            participants,
            messages,
            notes,
        } => sequence_view(ui, ecx, participants, messages, notes.as_deref()),
        BlockKind::NodeTable { title, rows } => node_table_view(ui, ecx, title, rows),
        BlockKind::Tree { nodes } => {
            tree_rows(ui, ecx, nodes, "");
        }
        BlockKind::Timeline {
            title,
            phases,
            items,
            ..
        } => timeline_view(ui, ecx, title.as_deref(), phases.as_deref(), items),
        BlockKind::ChapterHeader {
            title,
            kicker,
            reading_time,
            updated,
            version,
        } => chapter_view(
            ui,
            ecx,
            title,
            kicker.as_deref(),
            &[
                reading_time.as_deref(),
                updated.as_deref(),
                version.as_deref(),
            ],
        ),
        _ => {}
    }
}

/* ---------------- JSON source editor ---------------- */

fn json_edit(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &mut Document,
    addr: Address,
    id: egui::Id,
) {
    let t = ecx.t;
    let body_id = id.with("json-body");

    // Key interception only while the JSON body owns focus (the code-block
    // contract). A JSON draft owns Esc — it validates and commits, or
    // discards on a second press — so that one key never reaches the shared
    // key model; the rest of the buffer's block-level keys do.
    if ui.ctx().memory(|m| m.has_focus(body_id)) {
        if keys::consume_plain(ui, Key::Escape) {
            match serde_json::from_str::<BlockKind>(&st.json_draft) {
                Ok(kind) => {
                    if let Some(b) = doc.block_mut(addr) {
                        b.kind = kind;
                    }
                    st.changed = true;
                    st.json_err = None;
                    st.editing = false;
                    ui.ctx().memory_mut(|m| m.surrender_focus(body_id));
                }
                Err(e) => {
                    if st.json_err.is_some() && !st.json_dirty_since_err {
                        // Second Esc with no edits since the error: discard.
                        st.json_err = None;
                        st.editing = false;
                        ui.ctx().memory_mut(|m| m.surrender_focus(body_id));
                    } else {
                        st.json_err = Some(e.to_string());
                        st.json_dirty_since_err = false;
                    }
                }
            }
        } else {
            keys::buffer(ui, ecx, st, doc, addr, body_id);
        }
    }

    let type_name = doc
        .block(addr)
        .map(|b| kind_name(&b.kind))
        .unwrap_or("json");

    Frame::new()
        .fill(t.surface(Surface::Card))
        .stroke(Stroke::new(1.0, t.border.subtle))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 16.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(type_name)
                        .font(t.mono(t.type_scale.xs))
                        .color(t.text(TextRole::Tertiary)),
                );
                if let Some(err) = &st.json_err {
                    ui.label(
                        egui::RichText::new(err.as_str())
                            .font(t.mono(t.type_scale.xs))
                            .color(t.danger.base),
                    );
                }
            });
            ui.add_space(2.0);

            let mut layouter = |ui: &Ui,
                                buf: &dyn egui::TextBuffer,
                                wrap_width: f32|
             -> std::sync::Arc<egui::Galley> {
                let mut job = highlight_job(ui, t, buf.as_str(), "json", t.type_scale.sm);
                job.wrap.max_width = wrap_width;
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let out = egui::TextEdit::multiline(&mut st.json_draft)
                .id(body_id)
                .frame(egui::Frame::NONE)
                .font(t.mono(t.type_scale.sm))
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .lock_focus(true)
                .layouter(&mut layouter)
                .show(ui);
            if out.response.changed() {
                st.json_dirty_since_err = true;
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

    if st.pending_json == Some(addr) {
        ui.ctx().memory_mut(|m| m.request_focus(body_id));
        st.pending_json = None;
    }
}

/// The wire `type` name of a data kind (editor chip label).
pub(super) fn kind_name(kind: &BlockKind) -> &'static str {
    match kind {
        BlockKind::Image { .. } => "image",
        BlockKind::Video { .. } => "video",
        BlockKind::Math { .. } => "math",
        BlockKind::BarChart { .. } => "bar_chart",
        BlockKind::LineChart { .. } => "line_chart",
        BlockKind::PieChart { .. } => "pie_chart",
        BlockKind::Diagram { .. } => "diagram",
        BlockKind::SequenceDiagram { .. } => "sequence_diagram",
        BlockKind::StateDiagram { .. } => "state_diagram",
        BlockKind::NodeTable { .. } => "node_table",
        BlockKind::Tree { .. } => "tree",
        BlockKind::Timeline { .. } => "timeline",
        BlockKind::ChapterHeader { .. } => "chapter_header",
        _ => "json",
    }
}

/* ---------------- media ---------------- */

fn image_view(
    ui: &mut Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    src: &str,
    alt: &str,
    width: Option<f64>,
    height: Option<f64>,
) {
    #[cfg(feature = "images")]
    {
        if let Some(tex) = load_image(ui, st, src) {
            let tex_size = tex.size_vec2();
            let mut size = Vec2::new(
                width.map(|w| w as f32).unwrap_or(tex_size.x),
                height.map(|h| h as f32).unwrap_or(tex_size.y),
            );
            if width.is_none() && height.is_some() {
                size.x = tex_size.x * size.y / tex_size.y.max(1.0);
            }
            if height.is_none() && width.is_some() {
                size.y = tex_size.y * size.x / tex_size.x.max(1.0);
            }
            let avail = ui.available_width();
            if size.x > avail {
                size = size * (avail / size.x);
            }
            ui.image((tex.id(), size));
            if !alt.is_empty() {
                ui.label(
                    egui::RichText::new(alt)
                        .font(ecx.t.mono(ecx.t.type_scale.xs))
                        .color(ecx.t.text(TextRole::Tertiary)),
                );
            }
            return;
        }
    }
    #[cfg(not(feature = "images"))]
    let _ = (st, width, height);
    media_card(ui, ecx, "image", (!alt.is_empty()).then_some(alt), src);
}

/// Decode + upload `src` once, cached in the editor state. Failures cache
/// as `None` so a bad path doesn't retry every frame.
#[cfg(feature = "images")]
fn load_image<'s>(
    ui: &Ui,
    st: &'s mut BlockEditorState,
    src: &str,
) -> Option<&'s egui::TextureHandle> {
    if !st.img_cache.contains_key(src) {
        let tex = std::fs::read(src).ok().and_then(|bytes| {
            let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let color =
                egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            Some(ui.ctx().load_texture(
                format!("fblk-img:{src}"),
                color,
                egui::TextureOptions::LINEAR,
            ))
        });
        st.img_cache.insert(src.to_string(), tex);
    }
    st.img_cache.get(src).and_then(Option::as_ref)
}

/// Placeholder card: `[chip] primary — secondary`.
fn media_card(ui: &mut Ui, ecx: &mut Ecx, chip: &str, primary: Option<&str>, secondary: &str) {
    let t = ecx.t;
    Frame::new()
        .fill(t.surface(Surface::Card))
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(chip)
                        .font(t.mono(t.type_scale.xs))
                        .color(t.text(TextRole::Tertiary)),
                );
                if let Some(primary) = primary {
                    ui.label(
                        egui::RichText::new(primary)
                            .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm))
                            .color(t.text(TextRole::Primary)),
                    );
                }
                ui.label(
                    egui::RichText::new(secondary)
                        .font(t.mono(t.type_scale.xs))
                        .color(t.text(TextRole::Disabled)),
                );
            });
        });
}

fn math_view(ui: &mut Ui, ecx: &mut Ecx, tex: &str) {
    let t = ecx.t;
    Frame::new()
        .fill(t.surface(Surface::Card))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("math")
                        .font(t.mono(t.type_scale.xs))
                        .color(t.text(TextRole::Disabled)),
                );
            });
            ui.label(
                egui::RichText::new(tex)
                    .font(t.mono(t.type_scale.sm))
                    .color(t.text(TextRole::Secondary)),
            );
        });
}

/* ---------------- charts / diagrams ---------------- */

fn chart_title(ui: &mut Ui, ecx: &mut Ecx, title: Option<&str>) {
    if let Some(title) = title {
        ui.label(
            egui::RichText::new(title)
                .font(
                    ecx.t
                        .font(ui.ctx(), FontWeight::Medium, ecx.t.type_scale.sm),
                )
                .color(ecx.t.text(TextRole::Primary)),
        );
    }
}

fn sequence_view(
    ui: &mut Ui,
    ecx: &mut Ecx,
    participants: &[forge_blocks::SeqParticipant],
    messages: &[forge_blocks::SeqMessage],
    notes: Option<&[forge_blocks::SeqNote]>,
) {
    let t = ecx.t;
    if participants.is_empty() {
        return;
    }
    let n = participants.len();
    let avail = ui.available_width();
    let col_w = (avail / n as f32).clamp(90.0, 220.0);
    let total_w = col_w * n as f32;
    let head_h = 26.0;
    let row_h = 26.0;
    // One row per message plus one per note under its anchor.
    let notes = notes.unwrap_or_default();
    let rows = messages.len() + notes.len();
    let total_h = head_h + 10.0 + rows as f32 * row_h + 6.0;
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(total_w, total_h), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let cx = |i: usize| rect.min.x + i as f32 * col_w + col_w / 2.0;
    let col_of = |id: &str| participants.iter().position(|p| p.id == id);

    for (i, p) in participants.iter().enumerate() {
        let x = cx(i);
        painter.add(Shape::dashed_line(
            &[Pos2::new(x, rect.min.y + head_h), Pos2::new(x, rect.max.y)],
            Stroke::new(1.0, t.border.default),
            4.0,
            4.0,
        ));
        let head = Rect::from_center_size(
            Pos2::new(x, rect.min.y + head_h / 2.0),
            Vec2::new(col_w - 20.0, head_h - 4.0),
        );
        painter.rect(
            head,
            CornerRadius::same(t.radius.sm as u8),
            t.surface(Surface::Hover),
            Stroke::new(1.0, t.border.default),
            egui::StrokeKind::Inside,
        );
        painter.text(
            head.center(),
            Align2::CENTER_CENTER,
            p.name.as_deref().unwrap_or(&p.id),
            t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs),
            t.text(TextRole::Primary),
        );
    }

    let mut y = rect.min.y + head_h + 10.0 + row_h / 2.0;
    let mut note_iter: Vec<(usize, &str)> = notes
        .iter()
        .map(|nt| (nt.at as usize, nt.text.as_str()))
        .collect();
    note_iter.sort_by_key(|(at, _)| *at);
    let mut ni = 0usize;
    for (mi, m) in messages.iter().enumerate() {
        if let (Some(f), Some(to)) = (col_of(&m.from), col_of(&m.to)) {
            let (x0, x1) = (cx(f), cx(to));
            let dashed = matches!(m.kind, Some(MessageKind::Async) | Some(MessageKind::Reply));
            let stroke = Stroke::new(1.2, t.text(TextRole::Tertiary));
            if dashed {
                painter.add(Shape::dashed_line(
                    &[Pos2::new(x0, y), Pos2::new(x1, y)],
                    stroke,
                    5.0,
                    4.0,
                ));
            } else {
                painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], stroke);
            }
            let dir = if x1 >= x0 { 1.0 } else { -1.0 };
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(x1, y),
                    Pos2::new(x1 - dir * 7.0, y - 4.0),
                    Pos2::new(x1 - dir * 7.0, y + 4.0),
                ],
                t.accent.base,
                Stroke::NONE,
            ));
            if let Some(text_) = &m.text {
                painter.text(
                    Pos2::new((x0 + x1) / 2.0, y - 6.0),
                    Align2::CENTER_BOTTOM,
                    text_,
                    t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs),
                    t.text(TextRole::Secondary),
                );
            }
        }
        y += row_h;
        while ni < note_iter.len() && note_iter[ni].0 == mi {
            painter.text(
                Pos2::new(rect.min.x + 6.0, y),
                Align2::LEFT_CENTER,
                format!("▹ {}", note_iter[ni].1),
                t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs),
                t.text(TextRole::Disabled),
            );
            y += row_h;
            ni += 1;
        }
    }
}

/* ---------------- structure ---------------- */

fn node_table_view(ui: &mut Ui, ecx: &mut Ecx, title: &str, rows: &[forge_blocks::NodeTableRow]) {
    let t = ecx.t;
    let width = ui.available_width().min(340.0);
    Frame::new()
        .stroke(Stroke::new(1.0, t.border.default))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .show(ui, |ui| {
            ui.set_max_width(width);
            ui.set_min_width(width);
            if !title.is_empty() {
                Frame::new()
                    .fill(t.surface(Surface::Hover))
                    .inner_margin(Margin::symmetric(10, 5))
                    .show(ui, |ui| {
                        ui.set_min_width(width - 20.0);
                        ui.label(
                            egui::RichText::new(title)
                                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm))
                                .color(t.text(TextRole::Primary)),
                        );
                    });
            }
            for row in rows {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let dot = if row.key.is_some() {
                        ecx.t.accent.base
                    } else {
                        ecx.t.border.default
                    };
                    let (drect, _) = ui.allocate_exact_size(Vec2::new(8.0, 18.0), Sense::hover());
                    ui.painter().circle_filled(
                        Pos2::new(drect.center().x, drect.center().y),
                        3.0,
                        dot,
                    );
                    let style = InlineStyle {
                        size: t.type_scale.sm,
                        weight: FontWeight::Regular,
                        color: t.text(TextRole::Secondary),
                        italics: false,
                    };
                    let job = inline_job(ui, t, &row.md, style, f32::INFINITY);
                    ui.label(job);
                });
            }
            ui.add_space(4.0);
        });
}

fn tree_rows(ui: &mut Ui, ecx: &mut Ecx, nodes: &[forge_blocks::TreeNode], prefix: &str) {
    let t = ecx.t;
    for (i, node) in nodes.iter().enumerate() {
        let last = i + 1 == nodes.len();
        let branch = format!("{prefix}{}", if last { "└─ " } else { "├─ " });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(
                egui::RichText::new(branch)
                    .font(t.mono(t.type_scale.sm))
                    .color(t.text(TextRole::Disabled)),
            );
            if let Some(icon) = &node.icon {
                ui.label(
                    egui::RichText::new(format!("{icon} "))
                        .font(t.mono(t.type_scale.sm))
                        .color(t.text(TextRole::Tertiary)),
                );
            }
            ui.label(
                egui::RichText::new(&node.title)
                    .font(t.mono(t.type_scale.sm))
                    .color(t.text(TextRole::Secondary)),
            );
        });
        if let Some(children) = &node.children {
            let child_prefix = format!("{prefix}{}", if last { "   " } else { "│  " });
            tree_rows(ui, ecx, children, &child_prefix);
        }
    }
}

fn timeline_view(
    ui: &mut Ui,
    ecx: &mut Ecx,
    title: Option<&str>,
    phases: Option<&[forge_blocks::TimelinePhase]>,
    items: &[forge_blocks::TimelineItem],
) {
    let t = ecx.t;
    if let Some(title) = title {
        ui.label(
            egui::RichText::new(title)
                .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm))
                .color(t.text(TextRole::Primary)),
        );
    }
    if let Some(phases) = phases {
        if !phases.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (i, p) in phases.iter().enumerate() {
                    Frame::new()
                        .fill(t.surface(Surface::Hover))
                        .corner_radius(CornerRadius::same(99))
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&p.label)
                                        .font(t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs))
                                        .color(series_color(t, i)),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{} → {}", p.from, p.to))
                                        .font(t.mono(t.type_scale.xs))
                                        .color(t.text(TextRole::Disabled)),
                                );
                            });
                        });
                }
            });
        }
    }
    let row_h = 22.0;
    for (i, item) in items.iter().enumerate() {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::hover());
        let painter = ui.painter();
        let dot = Pos2::new(rect.min.x + 6.0, rect.center().y);
        painter.circle_filled(dot, 3.5, t.accent.base);
        if i + 1 < items.len() {
            painter.line_segment(
                [
                    Pos2::new(dot.x, dot.y + 5.0),
                    Pos2::new(dot.x, rect.max.y + row_h / 2.0 - 5.0),
                ],
                Stroke::new(1.0, t.border.default),
            );
        }
        painter.text(
            Pos2::new(rect.min.x + 18.0, rect.center().y),
            Align2::LEFT_CENTER,
            &item.label,
            t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm),
            t.text(TextRole::Primary),
        );
        let label_w = painter
            .layout_no_wrap(
                item.label.clone(),
                t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm),
                t.text(TextRole::Primary),
            )
            .size()
            .x;
        painter.text(
            Pos2::new(rect.min.x + 26.0 + label_w, rect.center().y),
            Align2::LEFT_CENTER,
            &item.on,
            t.mono(t.type_scale.xs),
            t.text(TextRole::Disabled),
        );
    }
}

fn chapter_view(
    ui: &mut Ui,
    ecx: &mut Ecx,
    title: &str,
    kicker: Option<&str>,
    meta: &[Option<&str>],
) {
    let t = ecx.t;
    if let Some(kicker) = kicker {
        ui.label(
            egui::RichText::new(kicker.to_uppercase())
                .font(t.mono(t.type_scale.xs))
                .color(t.accent.base),
        );
    }
    ui.label(
        egui::RichText::new(title)
            .font(t.font(ui.ctx(), FontWeight::SemiBold, t.type_scale.xl2))
            .color(t.text(TextRole::Primary)),
    );
    let parts: Vec<&str> = meta.iter().flatten().copied().collect();
    if !parts.is_empty() {
        ui.label(
            egui::RichText::new(parts.join(" · "))
                .font(t.mono(t.type_scale.xs))
                .color(t.text(TextRole::Tertiary)),
        );
    }
}
