//! Painters and height measurement for the data block kinds (image, video,
//! math, charts, diagrams, node table, tree, timeline, chapter header).
//! Charts and flowcharts reuse the kit widgets; the rest are box-drawing
//! painters. Everything sticks to theme colors so the 256-color quantizer
//! stays safe.

use forge_blocks::{
    BlockKind, ChartSeries, DiagramNode, MessageKind, SeqMessage, SeqNote, SeqParticipant,
    StateNode, StateTransition, TimelineItem, TimelinePhase, TreeNode,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::text;
use crate::theme::{series_color, Theme};
use crate::widgets::charts::{LineChart, LineSeries, PieChart, PieSlice};
use crate::widgets::specialty::{FlowEdge, FlowNode, Flowchart};

use super::render::{Painter, GUTTER};
use super::Editing;

/* ---------------- measurement ------------------------------------------- */

/// Rows a data block needs at content width `w` (display mode — the JSON
/// editor's height comes from the textarea in `measure_block`).
pub(super) fn data_height(kind: &BlockKind, _w: u16) -> usize {
    match kind {
        BlockKind::Image { .. } | BlockKind::Video { .. } => 3,
        BlockKind::Math { tex } => tex.lines().count().max(1) + 1,
        BlockKind::BarChart {
            title,
            categories,
            series,
            ..
        } => {
            title.is_some() as usize
                + categories.len().max(1) * series.len().max(1)
                + usize::from(series.len() > 1)
        }
        BlockKind::LineChart { title, series, .. } => {
            title.is_some() as usize + 10 + usize::from(series.len() > 1)
        }
        BlockKind::PieChart { title, .. } => title.is_some() as usize + 9,
        BlockKind::Diagram { nodes, edges, .. } => flow_height(
            &nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            &edge_pairs(edges.iter().map(|e| (e.from.as_str(), e.to.as_str()))),
        ),
        BlockKind::StateDiagram {
            states,
            transitions,
        } => flow_height(
            &states.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            &edge_pairs(transitions.iter().map(|t| (t.from.as_str(), t.to.as_str()))),
        ),
        BlockKind::SequenceDiagram {
            messages, notes, ..
        } => 3 + messages.len() * 2 + notes.as_ref().map_or(0, Vec::len) + 1,
        BlockKind::NodeTable { title, rows } => rows.len() + 2 + usize::from(!title.is_empty()),
        BlockKind::Tree { nodes } => count_tree(nodes).max(1),
        BlockKind::Timeline {
            title,
            phases,
            items,
            ..
        } => {
            title.is_some() as usize
                + usize::from(phases.as_ref().is_some_and(|p| !p.is_empty()))
                + items.len().max(1)
        }
        BlockKind::ChapterHeader {
            kicker,
            reading_time,
            updated,
            version,
            ..
        } => {
            kicker.is_some() as usize
                + 1
                + usize::from(reading_time.is_some() || updated.is_some() || version.is_some())
        }
        _ => 1,
    }
}

fn count_tree(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + n.children.as_deref().map_or(0, count_tree))
        .sum()
}

fn edge_pairs<'a>(iter: impl Iterator<Item = (&'a str, &'a str)>) -> Vec<(&'a str, &'a str)> {
    iter.collect()
}

fn flow_height(ids: &[&str], edges: &[(&str, &str)]) -> usize {
    let nodes: Vec<FlowNode> = ids.iter().map(|id| FlowNode::new(id, "")).collect();
    let fe: Vec<FlowEdge> = edges.iter().map(|(f, t)| FlowEdge::new(f, t)).collect();
    Flowchart::new(&nodes, &fe).required_height() as usize
}

/* ---------------- dispatch ---------------------------------------------- */

/// Paint a data block (display mode or its JSON source editor) into the
/// block's scratch buffer.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_data(
    p: &mut Painter,
    addr: forge_blocks::Address,
    kind: &BlockKind,
    buf: &mut Buffer,
    w: u16,
    h: u16,
    focused_here: bool,
) {
    let t = p.t;
    if focused_here {
        if let Editing::Data { ts, err, .. } = &*p.editing {
            let source = ts.value();
            let cursor = ts.cursor();
            let err = err.clone();
            paint_json_edit(p, addr, kind, &source, cursor, err.as_deref(), buf, w, h);
            return;
        }
    }
    let area = Rect::new(GUTTER, 0, w.saturating_sub(GUTTER), h);
    if area.width < 4 || area.height == 0 {
        return;
    }
    match kind {
        BlockKind::Image { src, alt, .. } => card(buf, area, t, "image", alt, src),
        BlockKind::Video { src, title, .. } => {
            card(buf, area, t, "▶ video", title.as_deref().unwrap_or(""), src)
        }
        BlockKind::Math { tex } => paint_math(buf, area, t, tex),
        BlockKind::BarChart {
            title,
            categories,
            series,
            y_max,
            ..
        } => paint_bars(buf, area, t, title.as_deref(), categories, series, *y_max),
        BlockKind::LineChart {
            title,
            categories,
            series,
            y_min,
            y_max,
            ..
        } => paint_line(
            buf,
            area,
            t,
            title.as_deref(),
            categories,
            series,
            *y_min,
            *y_max,
        ),
        BlockKind::PieChart { title, slices } => {
            let mut y = area.y;
            if let Some(title) = title {
                title_row(buf, area, t, title);
                y += 1;
            }
            let slices: Vec<PieSlice> = slices
                .iter()
                .map(|s| PieSlice::new(&s.label, s.value))
                .collect();
            let pie_area = Rect::new(
                area.x,
                y,
                area.width,
                area.height.saturating_sub(y - area.y),
            );
            PieChart::new(&slices).legend(true).render(pie_area, buf);
        }
        BlockKind::Diagram { nodes, edges, .. } => {
            let fnodes: Vec<FlowNode> = nodes
                .iter()
                .map(|n| FlowNode::new(&n.id, decorate_node(n)))
                .collect();
            let fedges: Vec<FlowEdge> = edges
                .iter()
                .map(|e| {
                    let fe = FlowEdge::new(&e.from, &e.to);
                    match &e.label {
                        Some(l) => fe.label(l),
                        None => fe,
                    }
                })
                .collect();
            Flowchart::new(&fnodes, &fedges).render(area, buf);
        }
        BlockKind::StateDiagram {
            states,
            transitions,
        } => paint_states(buf, area, states, transitions),
        BlockKind::SequenceDiagram {
            participants,
            messages,
            notes,
        } => paint_sequence(buf, area, t, participants, messages, notes.as_deref()),
        BlockKind::NodeTable { title, rows } => paint_node_table(buf, area, t, title, rows),
        BlockKind::Tree { nodes } => {
            let mut y = area.y;
            paint_tree(buf, area, t, nodes, "", &mut y);
        }
        BlockKind::Timeline {
            title,
            phases,
            items,
            ..
        } => paint_timeline(buf, area, t, title.as_deref(), phases.as_deref(), items),
        BlockKind::ChapterHeader {
            title,
            kicker,
            reading_time,
            updated,
            version,
        } => paint_chapter(
            buf,
            area,
            t,
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

/// The label shown inside a diagram node box (decision/terminator markers).
fn decorate_node(n: &DiagramNode) -> &str {
    // Shape variants aren't drawable at 3 rows; the kind shows via label
    // decoration painted by the flowchart (text only).
    &n.text
}

/* ---------------- JSON editor ------------------------------------------- */

#[allow(clippy::too_many_arguments)]
fn paint_json_edit(
    p: &mut Painter,
    addr: forge_blocks::Address,
    kind: &BlockKind,
    source: &str,
    cursor: (usize, usize),
    err: Option<&str>,
    buf: &mut Buffer,
    w: u16,
    h: u16,
) {
    let t = p.t;
    buf.set_style(
        Rect::new(GUTTER, 0, w.saturating_sub(GUTTER), h),
        Style::new().bg(t.bg[1]),
    );
    let label = kind_label(kind);
    buf.set_string(
        GUTTER + 1,
        0,
        text::truncate(label, w.saturating_sub(GUTTER + 1) as usize),
        Style::new().fg(t.fg[3]),
    );
    if let Some(err) = err {
        let x = GUTTER + 1 + text::width(label) as u16 + 2;
        if x < w {
            buf.set_string(
                x,
                0,
                text::truncate(err, w.saturating_sub(x) as usize),
                Style::new().fg(t.danger.base),
            );
        }
    }
    let styled = super::render::highlight_json(p, addr, source);
    let code_x = GUTTER + 1;
    let code_w = w.saturating_sub(code_x) as usize;
    for (li, spans) in styled.iter().enumerate() {
        let y = 1 + li as u16;
        if y >= h {
            break;
        }
        let mut x = code_x;
        let mut cells = 0usize;
        'spans: for (style, txt) in spans {
            for g in unicode_segmentation::UnicodeSegmentation::graphemes(txt.as_str(), true) {
                let gw = text::width(g);
                if cells + gw > code_w {
                    break 'spans;
                }
                buf.set_string(x, y, g, *style);
                x += gw as u16;
                cells += gw;
            }
        }
    }
    if p.widget_focused {
        let (row, colb) = cursor;
        let line = source.split('\n').nth(row).unwrap_or_default();
        let col = text::width(&line[..colb.min(line.len())]);
        let cx = code_x + col.min(code_w.saturating_sub(1)) as u16;
        let cy = 1 + row as u16;
        if cx < w && cy < h {
            buf.set_style(
                Rect::new(cx, cy, 1, 1),
                Style::new().add_modifier(Modifier::REVERSED),
            );
        }
    }
}

/// Slash-palette/editor label for a data kind's wire type name.
pub(super) fn kind_label(kind: &BlockKind) -> &'static str {
    match kind {
        BlockKind::Image { .. } => "image · json",
        BlockKind::Video { .. } => "video · json",
        BlockKind::Math { .. } => "math · json",
        BlockKind::BarChart { .. } => "bar_chart · json",
        BlockKind::LineChart { .. } => "line_chart · json",
        BlockKind::PieChart { .. } => "pie_chart · json",
        BlockKind::Diagram { .. } => "diagram · json",
        BlockKind::SequenceDiagram { .. } => "sequence_diagram · json",
        BlockKind::StateDiagram { .. } => "state_diagram · json",
        BlockKind::NodeTable { .. } => "node_table · json",
        BlockKind::Tree { .. } => "tree · json",
        BlockKind::Timeline { .. } => "timeline · json",
        BlockKind::ChapterHeader { .. } => "chapter_header · json",
        _ => "json",
    }
}

/* ---------------- painters ---------------------------------------------- */

fn title_row(buf: &mut Buffer, area: Rect, t: &Theme, title: &str) {
    buf.set_string(
        area.x,
        area.y,
        text::truncate(title, area.width as usize),
        Style::new().fg(t.fg[1]).add_modifier(Modifier::BOLD),
    );
}

/// Bordered placeholder card: `[chip] primary` over a dim secondary line.
fn card(buf: &mut Buffer, area: Rect, t: &Theme, chip: &str, primary: &str, secondary: &str) {
    let wu = area.width as usize;
    let dash = Style::new().fg(t.border.default);
    buf.set_string(area.x, area.y, format!("┌{}┐", "─".repeat(wu - 2)), dash);
    if area.height > 2 {
        buf.set_string(area.x, area.y + 1, "│", dash);
        buf.set_string(area.x + area.width - 1, area.y + 1, "│", dash);
        let label = if primary.is_empty() {
            format!("{chip}  {secondary}")
        } else {
            format!("{chip}  {primary} — {secondary}")
        };
        buf.set_string(
            area.x + 2,
            area.y + 1,
            text::truncate(&label, wu.saturating_sub(4)),
            Style::new().fg(t.fg[2]),
        );
        buf.set_string(
            area.x,
            area.y + 2,
            format!("└{}┘", "─".repeat(wu - 2)),
            dash,
        );
    }
}

fn paint_math(buf: &mut Buffer, area: Rect, t: &Theme, tex: &str) {
    buf.set_style(area, Style::new().bg(t.bg[1]));
    buf.set_string(area.x + 1, area.y, "math", Style::new().fg(t.fg[3]));
    for (i, line) in tex.lines().enumerate() {
        let y = area.y + 1 + i as u16;
        if y >= area.y + area.height {
            break;
        }
        buf.set_string(
            area.x + 1,
            y,
            text::truncate(line, area.width.saturating_sub(2) as usize),
            Style::new().fg(t.fg[1]),
        );
    }
}

fn fmt_val(v: f64) -> String {
    if (v.fract()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

fn paint_bars(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    title: Option<&str>,
    categories: &[String],
    series: &[ChartSeries],
    y_max: Option<f64>,
) {
    let mut y = area.y;
    if let Some(title) = title {
        title_row(buf, area, t, title);
        y += 1;
    }
    let multi = series.len() > 1;
    let max = y_max
        .unwrap_or_else(|| {
            series
                .iter()
                .flat_map(|s| s.values.iter().copied())
                .fold(0.0f64, f64::max)
        })
        .max(f64::EPSILON);
    let cat_w = categories.iter().map(|c| text::width(c)).max().unwrap_or(0);
    let name_w = if multi {
        series
            .iter()
            .map(|s| text::width(&s.name))
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    let val_w = 6usize;
    let bar_w = (area.width as usize)
        .saturating_sub(cat_w + 2 + if multi { name_w + 2 } else { 0 } + val_w)
        .max(4);

    for (ci, cat) in categories.iter().enumerate() {
        for (si, s) in series.iter().enumerate() {
            if y >= area.y + area.height {
                return;
            }
            let mut x = area.x;
            if si == 0 {
                buf.set_string(x, y, cat, Style::new().fg(t.fg[2]));
            }
            x += (cat_w + 2) as u16;
            if multi {
                buf.set_string(x, y, &s.name, Style::new().fg(t.fg[3]));
                x += (name_w + 2) as u16;
            }
            let v = s.values.get(ci).copied().unwrap_or(0.0);
            let cells = ((v / max) * bar_w as f64).round().max(0.0) as usize;
            let color = series_color(t, si);
            buf.set_string(x, y, "█".repeat(cells.min(bar_w)), Style::new().fg(color));
            buf.set_string(
                x + bar_w as u16 + 1,
                y,
                fmt_val(v),
                Style::new().fg(t.fg[2]),
            );
            y += 1;
        }
    }
    if multi && y < area.y + area.height {
        let mut x = area.x;
        for (si, s) in series.iter().enumerate() {
            let chip = format!("■ {}  ", s.name);
            let wch = text::width(&chip) as u16;
            if x + wch > area.x + area.width {
                break;
            }
            buf.set_string(x, y, "■", Style::new().fg(series_color(t, si)));
            buf.set_string(x + 2, y, &s.name, Style::new().fg(t.fg[2]));
            x += wch;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_line(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    title: Option<&str>,
    categories: &[String],
    series: &[ChartSeries],
    y_min: Option<f64>,
    y_max: Option<f64>,
) {
    let mut y = area.y;
    if let Some(title) = title {
        title_row(buf, area, t, title);
        y += 1;
    }
    let points: Vec<Vec<(f64, f64)>> = series
        .iter()
        .map(|s| {
            s.values
                .iter()
                .enumerate()
                .map(|(i, v)| (i as f64, *v))
                .collect()
        })
        .collect();
    let line_series: Vec<LineSeries> = series
        .iter()
        .zip(&points)
        .map(|(s, pts)| LineSeries::new(&s.name, pts))
        .collect();
    let lo = y_min.unwrap_or(0.0);
    let hi = y_max.unwrap_or_else(|| {
        series
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .fold(1.0f64, f64::max)
    });
    let n = categories
        .len()
        .max(series.iter().map(|s| s.values.len()).max().unwrap_or(1))
        .max(2);
    let chart_h = (area.height - (y - area.y)).saturating_sub(u16::from(series.len() > 1));
    LineChart::new(&line_series)
        .x_bounds([0.0, (n - 1) as f64])
        .y_bounds([lo, hi])
        .render(Rect::new(area.x, y, area.width, chart_h.max(3)), buf);
    if series.len() > 1 {
        let ly = y + chart_h.max(3);
        if ly < area.y + area.height {
            let mut x = area.x;
            for (si, s) in series.iter().enumerate() {
                let chip_w = text::width(&s.name) as u16 + 4;
                if x + chip_w > area.x + area.width {
                    break;
                }
                buf.set_string(x, ly, "■", Style::new().fg(series_color(t, si)));
                buf.set_string(x + 2, ly, &s.name, Style::new().fg(t.fg[2]));
                x += chip_w;
            }
        }
    }
}

fn paint_states(
    buf: &mut Buffer,
    area: Rect,
    states: &[StateNode],
    transitions: &[StateTransition],
) {
    let labels: Vec<String> = states
        .iter()
        .map(|s| {
            let name = s.name.as_deref().unwrap_or(&s.id);
            if s.initial == Some(true) {
                format!("● {name}")
            } else if s.is_final == Some(true) {
                format!("◉ {name}")
            } else {
                name.to_string()
            }
        })
        .collect();
    let edge_labels: Vec<String> = transitions
        .iter()
        .map(|tr| match (&tr.trigger, &tr.guard) {
            (Some(trig), Some(g)) => format!("{trig} [{g}]"),
            (Some(trig), None) => trig.clone(),
            (None, Some(g)) => format!("[{g}]"),
            (None, None) => String::new(),
        })
        .collect();
    let nodes: Vec<FlowNode> = states
        .iter()
        .zip(&labels)
        .map(|(s, l)| FlowNode::new(&s.id, l))
        .collect();
    let edges: Vec<FlowEdge> = transitions
        .iter()
        .zip(&edge_labels)
        .map(|(tr, l)| {
            let e = FlowEdge::new(&tr.from, &tr.to);
            if l.is_empty() {
                e
            } else {
                e.label(l)
            }
        })
        .collect();
    Flowchart::new(&nodes, &edges).render(area, buf);
}

fn paint_sequence(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    participants: &[SeqParticipant],
    messages: &[SeqMessage],
    notes: Option<&[SeqNote]>,
) {
    if participants.is_empty() {
        return;
    }
    let col_w = (area.width / participants.len().max(1) as u16).max(8);
    let cx = |i: usize| area.x + i as u16 * col_w + col_w / 2;
    let col_of = |id: &str| participants.iter().position(|p| p.id == id);
    let height = area.height;

    // Lifelines under the header row band.
    let life = Style::new().fg(t.border.default);
    for (i, _) in participants.iter().enumerate() {
        let x = cx(i);
        for y in 3..height {
            buf.set_string(x, area.y + y, "┆", life);
        }
    }
    // Participant boxes (3 rows).
    for (i, p) in participants.iter().enumerate() {
        let name = p.name.as_deref().unwrap_or(&p.id);
        let bw = (text::width(name) as u16 + 4).min(col_w);
        let bx = cx(i).saturating_sub(bw / 2).max(area.x);
        let border = Style::new().fg(t.border.default).bg(t.bg[1]);
        let wu = bw as usize;
        buf.set_string(bx, area.y, format!("┌{}┐", "─".repeat(wu - 2)), border);
        buf.set_string(bx, area.y + 1, "│", border);
        buf.set_string(bx + bw - 1, area.y + 1, "│", border);
        buf.set_string(
            bx + 2,
            area.y + 1,
            text::truncate(name, wu.saturating_sub(4)),
            Style::new()
                .fg(t.fg[0])
                .bg(t.bg[1])
                .add_modifier(Modifier::BOLD),
        );
        buf.set_string(bx, area.y + 2, format!("└{}┘", "─".repeat(wu - 2)), border);
    }
    // Messages: label above, arrow below, one message per 2 rows.
    let mut y = area.y + 4;
    let mut note_iter: Vec<(usize, &str)> = notes
        .unwrap_or_default()
        .iter()
        .map(|n| (n.at as usize, n.text.as_str()))
        .collect();
    note_iter.sort_by_key(|(at, _)| *at);
    let mut ni = 0usize;
    for (mi, m) in messages.iter().enumerate() {
        if y >= area.y + height {
            break;
        }
        let (Some(f), Some(to)) = (col_of(&m.from), col_of(&m.to)) else {
            continue;
        };
        let (x0, x1) = (cx(f), cx(to));
        let (lo, hi) = (x0.min(x1), x0.max(x1));
        let dashed = matches!(m.kind, Some(MessageKind::Async) | Some(MessageKind::Reply));
        let seg = if dashed { "╌" } else { "─" };
        let arrow_style = Style::new().fg(t.fg[2]);
        if let Some(text_) = &m.text {
            let label_x = lo + ((hi - lo) / 2).saturating_sub(text::width(text_) as u16 / 2);
            buf.set_string(
                label_x.max(area.x),
                y,
                text::truncate(text_, (hi - lo).max(4) as usize),
                Style::new().fg(t.fg[1]),
            );
        }
        let ay = y + 1;
        if ay < area.y + height && hi > lo {
            for x in lo + 1..hi {
                buf.set_string(x, ay, seg, arrow_style);
            }
            if x1 > x0 {
                buf.set_string(hi - 1, ay, "▶", Style::new().fg(t.accent.base));
            } else {
                buf.set_string(lo + 1, ay, "◀", Style::new().fg(t.accent.base));
            }
        }
        y += 2;
        while ni < note_iter.len() && note_iter[ni].0 == mi {
            if y < area.y + height {
                buf.set_string(
                    area.x + 1,
                    y,
                    text::truncate(
                        &format!("▹ {}", note_iter[ni].1),
                        area.width.saturating_sub(2) as usize,
                    ),
                    Style::new().fg(t.fg[3]).add_modifier(Modifier::ITALIC),
                );
                y += 1;
            }
            ni += 1;
        }
    }
}

fn paint_node_table(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    title: &str,
    rows: &[forge_blocks::NodeTableRow],
) {
    let w = area.width.min(46).max(8);
    let wu = w as usize;
    let border = Style::new().fg(t.border.default);
    let mut y = area.y;
    buf.set_string(area.x, y, format!("┌{}┐", "─".repeat(wu - 2)), border);
    y += 1;
    if !title.is_empty() {
        buf.set_string(area.x, y, "│", border);
        buf.set_string(area.x + w - 1, y, "│", border);
        buf.set_style(Rect::new(area.x + 1, y, w - 2, 1), Style::new().bg(t.bg[2]));
        buf.set_string(
            area.x + 2,
            y,
            text::truncate(title, wu.saturating_sub(4)),
            Style::new()
                .fg(t.fg[0])
                .bg(t.bg[2])
                .add_modifier(Modifier::BOLD),
        );
        y += 1;
    }
    for row in rows {
        if y >= area.y + area.height.saturating_sub(1) {
            break;
        }
        buf.set_string(area.x, y, "│", border);
        buf.set_string(area.x + w - 1, y, "│", border);
        let spans = forge_blocks::parse_inline(&row.md);
        let mut x = area.x + 2;
        let maxx = area.x + w - 2;
        'row: for s in &spans {
            let styled = super::render::style_span(s, Style::new().fg(t.fg[1]), t);
            for g in unicode_segmentation::UnicodeSegmentation::graphemes(
                s.text.replace('\n', " ").as_str(),
                true,
            ) {
                let gw = text::width(g) as u16;
                if x + gw > maxx {
                    break 'row;
                }
                buf.set_string(x, y, g, styled.style);
                x += gw;
            }
        }
        if row.key.is_some() {
            buf.set_string(area.x + 1, y, "•", Style::new().fg(t.accent.base));
        }
        y += 1;
    }
    if y < area.y + area.height {
        buf.set_string(area.x, y, format!("└{}┘", "─".repeat(wu - 2)), border);
    }
}

fn paint_tree(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    nodes: &[TreeNode],
    prefix: &str,
    y: &mut u16,
) {
    for (i, node) in nodes.iter().enumerate() {
        if *y >= area.y + area.height {
            return;
        }
        let last = i + 1 == nodes.len();
        let branch = format!("{prefix}{}", if last { "└─ " } else { "├─ " });
        buf.set_string(area.x, *y, &branch, Style::new().fg(t.fg[3]));
        let mut x = area.x + text::width(&branch) as u16;
        if let Some(icon) = &node.icon {
            buf.set_string(x, *y, icon, Style::new().fg(t.fg[2]));
            x += text::width(icon) as u16 + 1;
        }
        buf.set_string(
            x,
            *y,
            text::truncate(
                &node.title,
                (area.x + area.width).saturating_sub(x) as usize,
            ),
            Style::new().fg(t.fg[1]),
        );
        *y += 1;
        if let Some(children) = &node.children {
            let child_prefix = format!("{prefix}{}", if last { "   " } else { "│  " });
            paint_tree(buf, area, t, children, &child_prefix, y);
        }
    }
}

fn paint_timeline(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    title: Option<&str>,
    phases: Option<&[TimelinePhase]>,
    items: &[TimelineItem],
) {
    let mut y = area.y;
    if let Some(title) = title {
        title_row(buf, area, t, title);
        y += 1;
    }
    if let Some(phases) = phases {
        if !phases.is_empty() {
            let mut x = area.x;
            for p in phases {
                let chip = format!("▐{}▌ ", p.label);
                let cw = text::width(&chip) as u16;
                if x + cw > area.x + area.width {
                    break;
                }
                buf.set_string(x, y, &chip, Style::new().fg(t.fg[2]).bg(t.bg[1]));
                x += cw;
            }
            y += 1;
        }
    }
    let last = items.len().saturating_sub(1);
    for (i, item) in items.iter().enumerate() {
        if y >= area.y + area.height {
            break;
        }
        buf.set_string(area.x, y, "●", Style::new().fg(t.accent.base));
        if i != last {
            // Connector paints on the next row's dot column when there is one.
        }
        let label = format!(" {}  ", item.label);
        buf.set_string(area.x + 1, y, &label, Style::new().fg(t.fg[1]));
        let date_x = area.x + 1 + text::width(&label) as u16;
        buf.set_string(
            date_x,
            y,
            text::truncate(
                &item.on,
                (area.x + area.width).saturating_sub(date_x) as usize,
            ),
            Style::new().fg(t.fg[3]),
        );
        y += 1;
    }
}

fn paint_chapter(
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    title: &str,
    kicker: Option<&str>,
    meta: &[Option<&str>],
) {
    let mut y = area.y;
    if let Some(kicker) = kicker {
        buf.set_string(
            area.x,
            y,
            text::truncate(&kicker.to_uppercase(), area.width as usize),
            Style::new().fg(t.accent.base).add_modifier(Modifier::BOLD),
        );
        y += 1;
    }
    buf.set_string(
        area.x,
        y,
        text::truncate(title, area.width as usize),
        Style::new()
            .fg(t.fg[0])
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    );
    y += 1;
    let parts: Vec<&str> = meta.iter().flatten().copied().collect();
    if !parts.is_empty() && y < area.y + area.height {
        buf.set_string(
            area.x,
            y,
            text::truncate(&parts.join(" · "), area.width as usize),
            Style::new().fg(t.fg[3]),
        );
    }
}
