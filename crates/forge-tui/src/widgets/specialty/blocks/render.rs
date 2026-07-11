//! Layout and painting for the block editor: per-block measurement at the
//! viewport width, the virtual-row layout the scroll runs over, and the
//! per-kind painters. Blocks paint into a scratch buffer sized to the block
//! and the visible rows blit into the frame, so partially scrolled blocks
//! clip cleanly.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use forge_blocks::{parse_inline, wrap_spans, Address, Block, BlockKind, Document, InlineSpan};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme as SynTheme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::SyntaxSet;

use crate::text;
use crate::theme::Theme;

use super::{list_ordinal, tone_severity, CustomBlock, Editing};

/// Syntect output for one source: styled runs per line.
pub(super) type StyledLines = Vec<Vec<(Style, String)>>;
/// Per-block-id highlight cache keyed by a content hash.
pub(super) type CodeCache = HashMap<String, (u64, StyledLines)>;

/// Blank rows between sibling blocks.
pub(super) const GAP: usize = 1;
/// Left gutter (focus rail column plus one spacer) inside every block rect.
pub(super) const GUTTER: u16 = 2;

/// One renderable region: a root block or a column-cell block. Columns
/// containers get a `container` slot spanning all their cells (hit-testing
/// and selection tint only — cells paint the content).
#[derive(Clone, Copy, Debug)]
pub(super) struct Slot {
    pub addr: Address,
    pub container: bool,
    /// Virtual row offset from the top of the document.
    pub y: usize,
    /// Column offset from the left edge of the editor area.
    pub x: u16,
    pub w: u16,
    pub h: usize,
}

/// Cached hit region after a render: the on-screen clipped rect plus how
/// many block rows were scrolled off above it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Hit {
    pub addr: Address,
    pub container: bool,
    pub rect: Rect,
    pub top_skip: u16,
}

/// Everything the measurement/painting passes need from the editor state,
/// split into disjoint borrows.
pub(super) struct Painter<'a> {
    pub doc: &'a Document,
    pub focus: Option<Address>,
    pub editing: &'a mut Editing,
    pub table_cell: Option<(usize, usize)>,
    pub custom_active: bool,
    pub custom: &'a mut Vec<Box<dyn CustomBlock>>,
    pub code_cache: &'a mut CodeCache,
    pub read_only: bool,
    pub widget_focused: bool,
    pub t: &'a Theme,
}

/* ---------------- geometry ---------------------------------------------- */

/// Split `width` into per-column (x, w) pairs with one-cell gutters, honoring
/// the ratio weights.
pub(super) fn column_rects(width: u16, ratios: &[f32]) -> Vec<(u16, u16)> {
    let n = ratios.len().max(1) as u16;
    let avail = width.saturating_sub(n.saturating_sub(1));
    let sum: f32 = ratios.iter().sum::<f32>().max(f32::EPSILON);
    let mut out = Vec::new();
    let mut x = 0u16;
    let mut used = 0u16;
    for (i, r) in ratios.iter().enumerate() {
        let w = if i + 1 == ratios.len() {
            avail.saturating_sub(used)
        } else {
            (((avail as f32) * (r / sum)).round() as u16).min(avail.saturating_sub(used))
        };
        out.push((x, w.max(1)));
        x += w.max(1) + 1;
        used += w.max(1);
    }
    out
}

/// Marker text for a list item ("• ", "3. ", "[x] ").
pub(super) fn list_marker(
    style: forge_blocks::ListStyle,
    checked: Option<bool>,
    ordinal: usize,
) -> String {
    match style {
        forge_blocks::ListStyle::Bullet => "• ".to_string(),
        forge_blocks::ListStyle::Number => format!("{ordinal}. "),
        forge_blocks::ListStyle::Todo => {
            if checked == Some(true) {
                "[x] ".to_string()
            } else {
                "[ ] ".to_string()
            }
        }
    }
}

/// Prefix (marker/gutter) width in cells for a text kind — the content
/// column starts at `GUTTER + prefix` for every wrapped row. The admonition
/// value is the body inset inside its tint band.
pub(super) fn text_prefix_width(doc: &Document, addr: Address, kind: &BlockKind) -> u16 {
    match kind {
        BlockKind::ListItem {
            style,
            checked,
            indent,
            ..
        } => {
            let marker = list_marker(*style, *checked, list_ordinal(doc, addr));
            (*indent as u16) * 2 + text::width(&marker) as u16
        }
        BlockKind::Quote { .. } => 2,
        BlockKind::Admonition { .. } => 4,
        _ => 0,
    }
}

fn admonition_extra(kind: &BlockKind) -> usize {
    usize::from(matches!(kind, BlockKind::Admonition { .. }))
}

/* ---------------- measurement ------------------------------------------- */

/// Rows a leaf block occupies at width `w` (including its own gutter). When
/// the block is being edited its raw source drives the height, and the
/// active `WrapEdit` learns the content width as a side effect.
pub(super) fn measure_block(p: &mut Painter, block: &Block, addr: Address, w: u16) -> usize {
    let editing_here = p.focus == Some(addr) && !p.read_only;
    match &block.kind {
        BlockKind::Divider => 1,
        BlockKind::Code { code, .. } => {
            if editing_here {
                if let Editing::Code(ts) = &*p.editing {
                    return ts.line_count() + 1;
                }
            }
            code.lines().count().max(1) + 1
        }
        BlockKind::Table { rows, .. } => rows.len() + 4,
        BlockKind::Custom { kind, data } => {
            match p.custom.iter().find(|c| c.kind() == kind.as_str()) {
                Some(imp) => imp.height(data, w.saturating_sub(GUTTER), p.t).max(1) as usize,
                None => 3,
            }
        }
        BlockKind::Columns { .. } => 1, // containers are measured in layout()
        kind => {
            let prefix = text_prefix_width(p.doc, addr, kind);
            let content_w = w.saturating_sub(GUTTER + prefix).max(1) as usize;
            let extra = admonition_extra(kind);
            if editing_here {
                if let Editing::Text(we) = &mut *p.editing {
                    we.set_width(content_w);
                    return we.rows().max(1) + extra;
                }
            }
            let md = kind.md().unwrap_or_default();
            wrap_spans(&parse_inline(md), content_w).len().max(1) + extra
        }
    }
}

/// Lay the document out at `width`: leaf slots (plus container slots for
/// columns) and the total virtual height.
pub(super) fn layout(p: &mut Painter, width: u16) -> (Vec<Slot>, usize) {
    let mut slots = Vec::new();
    let mut y = 0usize;
    let doc = p.doc;
    for (i, block) in doc.blocks.iter().enumerate() {
        match &block.kind {
            BlockKind::Columns { columns } => {
                let ratios: Vec<f32> = columns.iter().map(|c| c.ratio).collect();
                let rects = column_rects(width, &ratios);
                let mut max_h = 0usize;
                for (c, col) in columns.iter().enumerate() {
                    let (cx, cw) = rects[c];
                    let mut cy = 0usize;
                    for (j, b) in col.blocks.iter().enumerate() {
                        let addr = Address::Cell {
                            root: i,
                            col: c,
                            idx: j,
                        };
                        let h = measure_block(p, b, addr, cw);
                        slots.push(Slot {
                            addr,
                            container: false,
                            y: y + cy,
                            x: cx,
                            w: cw,
                            h,
                        });
                        cy += h + GAP;
                    }
                    max_h = max_h.max(cy.saturating_sub(GAP));
                }
                slots.push(Slot {
                    addr: Address::Root(i),
                    container: true,
                    y,
                    x: 0,
                    w: width,
                    h: max_h,
                });
                y += max_h + GAP;
            }
            _ => {
                let addr = Address::Root(i);
                let h = measure_block(p, block, addr, width);
                slots.push(Slot {
                    addr,
                    container: false,
                    y,
                    x: 0,
                    w: width,
                    h,
                });
                y += h + GAP;
            }
        }
    }
    (slots, y.saturating_sub(GAP))
}

/* ---------------- inline span styling ----------------------------------- */

/// Map one shared inline span onto a ratatui span: strong bold, emphasis
/// italic, strike crossed-out, code tinted on `bg[3]`, links accent
/// underlined. Spans leave the background alone so tint bands show through.
pub(super) fn style_span(s: &InlineSpan, base: Style, t: &Theme) -> Span<'static> {
    let mut st = base;
    if s.strong {
        st = st.fg(t.fg[0]).add_modifier(Modifier::BOLD);
    }
    if s.emphasis {
        st = st.add_modifier(Modifier::ITALIC);
    }
    if s.strike {
        st = st.add_modifier(Modifier::CROSSED_OUT);
    }
    if s.code {
        st = st.fg(t.accent.fg).bg(t.bg[3]);
    }
    if s.link.is_some() {
        st = st.fg(t.accent.base).add_modifier(Modifier::UNDERLINED);
    }
    Span::styled(s.text.clone(), st)
}

fn heading_style(level: u8, t: &Theme) -> Style {
    match level {
        1 => Style::new()
            .fg(t.fg[0])
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        2 => Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD),
        _ => Style::new().fg(t.fg[1]).add_modifier(Modifier::BOLD),
    }
}

/* ---------------- painting ---------------------------------------------- */

/// Paint one leaf block into a scratch buffer of exactly (w, h).
pub(super) fn paint_block(
    p: &mut Painter,
    block: &Block,
    addr: Address,
    buf: &mut Buffer,
    w: u16,
    h: u16,
    container_selected: bool,
) {
    let t = p.t;
    let focused_here = p.focus == Some(addr) && !p.read_only;
    let editing_here = focused_here && (!matches!(*p.editing, Editing::None) || p.custom_active);
    let selected = focused_here && !editing_here;

    if selected || container_selected {
        buf.set_style(Rect::new(0, 0, w, h), Style::new().bg(t.accent.bg));
    }
    if focused_here {
        for dy in 0..h {
            buf.set_string(0, dy, "▎", Style::new().fg(t.accent.base));
        }
    }
    let cw = w.saturating_sub(GUTTER);
    if cw == 0 {
        return;
    }

    match &block.kind {
        BlockKind::Divider => {
            buf.set_string(GUTTER, 0, "─".repeat(cw as usize), Style::new().fg(t.fg[3]));
        }
        BlockKind::Code { lang, code } => {
            paint_code(p, addr, lang, code, buf, w, h, focused_here);
        }
        BlockKind::Table { header, rows } => {
            paint_table(p, addr, header, rows, buf, w, h, focused_here);
        }
        BlockKind::Custom { kind, data } => {
            let area = Rect::new(GUTTER, 0, cw, h);
            match p.custom.iter_mut().find(|c| c.kind() == kind.as_str()) {
                Some(imp) => {
                    let active = focused_here && p.custom_active;
                    imp.render(data, area, buf, focused_here || active, t)
                }
                None => paint_unknown_custom(kind, area, buf, t),
            }
        }
        BlockKind::Columns { .. } => {}
        kind => paint_text_block(p, addr, kind, buf, w, h, focused_here),
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_text_block(
    p: &mut Painter,
    addr: Address,
    kind: &BlockKind,
    buf: &mut Buffer,
    w: u16,
    h: u16,
    focused_here: bool,
) {
    let t = p.t;
    let widget_focused = p.widget_focused;
    let prefix_w = text_prefix_width(p.doc, addr, kind);
    let content_x = GUTTER + prefix_w;
    let content_w = w.saturating_sub(content_x).max(1) as usize;
    let mut body_y = 0u16;

    // Kind chrome: markers, quote bar, admonition band + title.
    let base = match kind {
        BlockKind::Heading { level, .. } => heading_style(*level, t),
        BlockKind::Quote { .. } => {
            for dy in 0..h {
                buf.set_string(GUTTER, dy, "▎ ", Style::new().fg(t.border.default));
            }
            Style::new().fg(t.fg[1])
        }
        BlockKind::ListItem {
            style,
            checked,
            indent,
            ..
        } => {
            let marker = list_marker(*style, *checked, list_ordinal(p.doc, addr));
            buf.set_string(
                GUTTER + (*indent as u16) * 2,
                0,
                marker,
                Style::new().fg(t.fg[2]),
            );
            Style::new().fg(t.fg[1])
        }
        BlockKind::Admonition { tone, title, .. } => {
            let tri = t.severity(tone_severity(*tone));
            buf.set_style(Rect::new(GUTTER, 0, w - GUTTER, h), Style::new().bg(tri.bg));
            for dy in 0..h {
                buf.set_string(GUTTER, dy, "▎", Style::new().fg(tri.base));
            }
            let title = if title.is_empty() { "Note" } else { title };
            buf.set_string(
                GUTTER + 2,
                0,
                text::truncate(title, w.saturating_sub(GUTTER + 2) as usize),
                Style::new().fg(tri.fg).add_modifier(Modifier::BOLD),
            );
            body_y = 1;
            Style::new().fg(t.fg[1])
        }
        _ => Style::new().fg(t.fg[1]),
    };

    let editing_text = focused_here && matches!(*p.editing, Editing::Text(_));
    if editing_text {
        // Raw source, soft-wrapped exactly like the measurement pass.
        let Editing::Text(we) = &mut *p.editing else {
            return;
        };
        we.set_width(content_w);
        let src = we.src().to_string();
        for (i, (s, e)) in we.lines().into_iter().enumerate() {
            let y = body_y + i as u16;
            if y >= h {
                break;
            }
            buf.set_string(
                content_x,
                y,
                text::truncate(&src[s..e], content_w),
                Style::new().fg(t.fg[0]),
            );
        }
        if widget_focused {
            let (row, col) = we.pos();
            let cx = content_x + (col.min(content_w.saturating_sub(1))) as u16;
            let cy = body_y + row as u16;
            if cx < w && cy < h {
                buf.set_style(
                    Rect::new(cx, cy, 1, 1),
                    Style::new().add_modifier(Modifier::REVERSED),
                );
            }
        }
    } else {
        let md = kind.md().unwrap_or_default();
        let lines = wrap_spans(&parse_inline(md), content_w);
        for (i, spans) in lines.into_iter().enumerate() {
            let y = body_y + i as u16;
            if y >= h {
                break;
            }
            let line = Line::from(
                spans
                    .iter()
                    .map(|s| style_span(s, base, t))
                    .collect::<Vec<_>>(),
            );
            buf.set_line(content_x, y, &line, content_w as u16);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_code(
    p: &mut Painter,
    addr: Address,
    lang: &str,
    code: &str,
    buf: &mut Buffer,
    w: u16,
    h: u16,
    focused_here: bool,
) {
    let t = p.t;
    let widget_focused = p.widget_focused;
    buf.set_style(
        Rect::new(GUTTER, 0, w - GUTTER, h),
        Style::new().bg(t.bg[1]),
    );
    let label = if lang.is_empty() { "code" } else { lang };
    buf.set_string(
        GUTTER + 1,
        0,
        text::truncate(label, w.saturating_sub(GUTTER + 1) as usize),
        Style::new().fg(t.fg[3]),
    );
    let (source, cursor) = if focused_here {
        match &*p.editing {
            Editing::Code(ts) => (ts.value(), Some(ts.cursor())),
            _ => (code.to_string(), None),
        }
    } else {
        (code.to_string(), None)
    };
    let id = match p.doc.block(addr) {
        Some(b) => b.id.clone(),
        None => return,
    };
    let styled = highlight_cached(&mut *p.code_cache, &id, lang, &source, t);
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
    if widget_focused {
        if let Some((row, colb)) = cursor {
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
}

#[allow(clippy::too_many_arguments)]
fn paint_table(
    p: &mut Painter,
    _addr: Address,
    header: &[String],
    rows: &[Vec<String>],
    buf: &mut Buffer,
    w: u16,
    h: u16,
    focused_here: bool,
) {
    let t = p.t;
    let ncols = header.len().max(1);
    let total = w.saturating_sub(GUTTER) as usize;
    if total < ncols * 2 + ncols + 1 {
        buf.set_string(
            GUTTER,
            0,
            text::truncate("[table]", total),
            Style::new().fg(t.fg[3]),
        );
        return;
    }
    let cell_w = (total - (ncols + 1)) / ncols;
    let grid = Style::new().fg(t.border.default);
    let x0 = GUTTER;

    let hline = |left: &str, mid: &str, right: &str| {
        let mut s = String::from(left);
        for c in 0..ncols {
            s.push_str(&"─".repeat(cell_w));
            s.push_str(if c + 1 == ncols { right } else { mid });
        }
        s
    };
    let row_count = rows.len() + 4;
    for (vy, content) in [(0u16, hline("┌", "┬", "┐"))]
        .into_iter()
        .chain(std::iter::once((2, hline("├", "┼", "┤"))))
        .chain(std::iter::once((
            (row_count - 1) as u16,
            hline("└", "┴", "┘"),
        )))
    {
        if vy < h {
            buf.set_string(x0, vy, &content, grid);
        }
    }

    let widget_focused = p.widget_focused;
    let table_cell = p.table_cell;
    let entered = focused_here && table_cell.is_some();
    let editing_cell = if entered {
        match &*p.editing {
            Editing::Cell(we) => Some((
                table_cell.unwrap_or_default(),
                we.src().to_string(),
                we.pos(),
            )),
            _ => None,
        }
    } else {
        None
    };

    // Display row r: 0 = header at y 1, body row i at y 3 + i.
    let mut draw_row = |cells: &[String], y: u16, display_r: usize, bold: bool| {
        if y >= h {
            return;
        }
        let mut x = x0;
        for c in 0..ncols {
            buf.set_string(x, y, "│", grid);
            x += 1;
            let inner = Rect::new(x, y, cell_w as u16, 1);
            let is_focus = entered && table_cell == Some((display_r, c));
            if is_focus {
                buf.set_style(inner, Style::new().bg(t.bg[3]));
            }
            let base = if bold {
                Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(t.fg[1])
            };
            match &editing_cell {
                Some(((er, ec), src, (_, col))) if (*er, *ec) == (display_r, c) => {
                    buf.set_string(x, y, text::fit(src, cell_w), Style::new().fg(t.fg[0]));
                    if widget_focused {
                        let cx = x + (*col).min(cell_w.saturating_sub(1)) as u16;
                        buf.set_style(
                            Rect::new(cx, y, 1, 1),
                            Style::new().add_modifier(Modifier::REVERSED),
                        );
                    }
                }
                _ => {
                    let text_cell = cells.get(c).map(String::as_str).unwrap_or("");
                    let spans = parse_inline(text_cell);
                    let mut cx = x;
                    let mut used = 0usize;
                    'cell: for s in &spans {
                        let styled = style_span(s, base, t);
                        for g in unicode_segmentation::UnicodeSegmentation::graphemes(
                            s.text.replace('\n', " ").as_str(),
                            true,
                        ) {
                            let gw = text::width(g);
                            if used + gw > cell_w {
                                break 'cell;
                            }
                            buf.set_string(cx, y, g, styled.style);
                            cx += gw as u16;
                            used += gw;
                        }
                    }
                }
            }
            x += cell_w as u16;
        }
        buf.set_string(x, y, "│", grid);
    };

    draw_row(header, 1, 0, true);
    for (i, row) in rows.iter().enumerate() {
        draw_row(row, 3 + i as u16, i + 1, false);
    }
}

fn paint_unknown_custom(kind: &str, area: Rect, buf: &mut Buffer, t: &Theme) {
    if area.width < 2 || area.height == 0 {
        return;
    }
    let dash = Style::new().fg(t.border.default);
    let wu = area.width as usize;
    buf.set_string(area.x, area.y, format!("┌{}┐", "┄".repeat(wu - 2)), dash);
    for dy in 1..area.height.saturating_sub(1) {
        buf.set_string(area.x, area.y + dy, "┆", dash);
        buf.set_string(area.x + area.width - 1, area.y + dy, "┆", dash);
    }
    if area.height > 1 {
        buf.set_string(
            area.x,
            area.y + area.height - 1,
            format!("└{}┘", "┄".repeat(wu - 2)),
            dash,
        );
    }
    if area.height > 2 {
        buf.set_string(
            area.x + 2,
            area.y + 1,
            text::truncate(&format!("custom: {kind}"), wu.saturating_sub(4)),
            Style::new().fg(t.fg[2]),
        );
    }
}

/* ---------------- syntect ------------------------------------------------ */

fn syn_color(c: Color) -> Option<SynColor> {
    match c {
        Color::Rgb(r, g, b) => Some(SynColor { r, g, b, a: 255 }),
        _ => None,
    }
}

fn syntax_set() -> &'static SyntaxSet {
    static SET: std::sync::OnceLock<SyntaxSet> = std::sync::OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// The CodeView scope mapping rebuilt for the editor (CodeView keeps its
/// theme private). Truecolor tokens required; quantized themes return `None`
/// and code falls back to plain `fg[1]`.
fn forge_syn_theme(t: &Theme) -> Option<SynTheme> {
    let fg = syn_color(t.fg[0])?;
    let item = |scopes: &str, color: Color, bold: bool| -> Option<ThemeItem> {
        let selectors: ScopeSelectors = scopes.parse().ok()?;
        Some(ThemeItem {
            scope: selectors,
            style: StyleModifier {
                foreground: syn_color(color),
                background: None,
                font_style: bold.then_some(FontStyle::BOLD),
            },
        })
    };
    let scopes = [
        (
            "keyword, storage.modifier, storage.type.function, storage.type.class",
            t.accent.fg,
            false,
        ),
        ("string, punctuation.definition.string", t.success.fg, false),
        (
            "constant.numeric, constant.language, constant.character",
            t.info.fg,
            false,
        ),
        ("comment, punctuation.definition.comment", t.fg[3], false),
        (
            "entity.name.type, entity.name.class, support.type, support.class, storage.type",
            t.warning.fg,
            false,
        ),
        (
            "entity.other.attribute-name, support.function, meta.property-name, variable.other.member",
            t.info.fg,
            false,
        ),
        ("entity.name.function", t.fg[0], true),
        ("punctuation, keyword.operator", t.fg[2], false),
        ("variable", t.fg[0], false),
    ];
    let mut theme = SynTheme {
        settings: ThemeSettings {
            foreground: Some(fg),
            ..Default::default()
        },
        ..Default::default()
    };
    for (sel, color, bold) in scopes {
        theme.scopes.push(item(sel, color, bold)?);
    }
    Some(theme)
}

fn highlight_lines(lang: &str, source: &str, t: &Theme) -> StyledLines {
    let ss = syntax_set();
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let syn_theme = forge_syn_theme(t);
    let mut highlighter = syn_theme.as_ref().map(|th| HighlightLines::new(syntax, th));
    let mut out = Vec::new();
    for line in source.split('\n') {
        match &mut highlighter {
            Some(hl) => {
                let spans = hl
                    .highlight_line(line, ss)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(s, txt)| {
                        let mut style = Style::new().fg(Color::Rgb(
                            s.foreground.r,
                            s.foreground.g,
                            s.foreground.b,
                        ));
                        if s.font_style.contains(FontStyle::BOLD) {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        (style.bg(t.bg[1]), txt.to_owned())
                    })
                    .collect();
                out.push(spans);
            }
            None => out.push(vec![(
                Style::new().fg(t.fg[1]).bg(t.bg[1]),
                line.to_owned(),
            )]),
        }
    }
    out
}

/// Highlighted lines cached per block id, keyed by a content hash so edits
/// and theme flips invalidate.
fn highlight_cached<'c>(
    cache: &'c mut CodeCache,
    id: &str,
    lang: &str,
    source: &str,
    t: &Theme,
) -> &'c StyledLines {
    let mut hasher = DefaultHasher::new();
    lang.hash(&mut hasher);
    source.hash(&mut hasher);
    t.name.hash(&mut hasher);
    matches!(t.fg[0], Color::Rgb(..)).hash(&mut hasher);
    let key = hasher.finish();
    if cache.get(id).map(|e| e.0) != Some(key) {
        let lines = highlight_lines(lang, source, t);
        cache.insert(id.to_string(), (key, lines));
    }
    &cache.get(id).expect("just inserted").1
}
