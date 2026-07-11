//! Markdown rendering (cargo feature `markdown`, pulldown-cmark) — the egui
//! sibling of `@forge/markdown` and forge-tui's markdown widget. The source
//! is parsed into a small block model (headings, paragraphs of styled
//! inline runs, lists, quotes, fenced code, tables, rules) and rendered with
//! Forge tokens. Raw HTML is never rendered, and link schemes are restricted
//! to http(s)/mailto before becoming hyperlinks (parity with the web kit's
//! XSS-safe renderer).

use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Separator;
use egui::{CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/* ---------------- block model (unit-testable) ---------------- */

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MdSpan {
    Text {
        text: String,
        strong: bool,
        emphasis: bool,
        strike: bool,
    },
    Code(String),
    Link {
        text: String,
        url: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MdBlock {
    Heading {
        level: u8,
        spans: Vec<MdSpan>,
    },
    Paragraph(Vec<MdSpan>),
    CodeBlock {
        lang: String,
        code: String,
    },
    Quote(Vec<MdBlock>),
    List {
        start: Option<u64>,
        items: Vec<Vec<MdBlock>>,
    },
    Rule,
    Table {
        header: Vec<Vec<MdSpan>>,
        rows: Vec<Vec<Vec<MdSpan>>>,
    },
}

/// Allow only http(s) and mailto link schemes (web parity: everything else —
/// `javascript:`, `data:`, relative paths — renders as plain text).
pub(crate) fn safe_url(url: &str) -> Option<&str> {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
    {
        Some(trimmed)
    } else {
        None
    }
}

enum Container {
    Quote(Vec<MdBlock>),
    List {
        start: Option<u64>,
        items: Vec<Vec<MdBlock>>,
    },
    Item(Vec<MdBlock>),
}

#[derive(Default)]
struct TableCx {
    header: Vec<Vec<MdSpan>>,
    rows: Vec<Vec<Vec<MdSpan>>>,
    row: Vec<Vec<MdSpan>>,
    cell: Vec<MdSpan>,
    in_head: bool,
}

#[derive(Default)]
struct ParseCx {
    root: Vec<MdBlock>,
    stack: Vec<Container>,
    spans: Vec<MdSpan>,
    strong: u32,
    emphasis: u32,
    strike: u32,
    /// `(url, accumulated text)` while inside a link.
    link: Option<(String, String)>,
    heading: Option<u8>,
    /// `(lang, buffer)` while inside a fenced/indented code block.
    code: Option<(String, String)>,
    table: Option<TableCx>,
}

impl ParseCx {
    fn push_block(&mut self, block: MdBlock) {
        match self.stack.last_mut() {
            Some(Container::Quote(blocks)) | Some(Container::Item(blocks)) => blocks.push(block),
            // Blocks directly inside a List (malformed) go to the last item.
            Some(Container::List { items, .. }) => match items.last_mut() {
                Some(item) => item.push(block),
                None => items.push(vec![block]),
            },
            None => self.root.push(block),
        }
    }

    fn flush_paragraph(&mut self) {
        if !self.spans.is_empty() {
            let spans = std::mem::take(&mut self.spans);
            self.push_block(MdBlock::Paragraph(spans));
        }
    }

    fn push_span(&mut self, span: MdSpan) {
        match &mut self.table {
            Some(table) => table.cell.push(span),
            None => self.spans.push(span),
        }
    }

    fn text(&mut self, s: &str) {
        if let Some((_, buf)) = &mut self.code {
            buf.push_str(s);
            return;
        }
        if let Some((_, text)) = &mut self.link {
            text.push_str(s);
            return;
        }
        self.push_span(MdSpan::Text {
            text: s.to_owned(),
            strong: self.strong > 0,
            emphasis: self.emphasis > 0,
            strike: self.strike > 0,
        });
    }
}

/// Parse markdown into the block model.
pub(crate) fn parse(source: &str) -> Vec<MdBlock> {
    let mut cx = ParseCx::default();
    let parser = Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    );
    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                cx.flush_paragraph();
                cx.heading = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                let level = cx.heading.take().unwrap_or(1);
                let spans = std::mem::take(&mut cx.spans);
                cx.push_block(MdBlock::Heading { level, spans });
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => cx.flush_paragraph(),
            Event::Start(Tag::BlockQuote(_)) => {
                cx.flush_paragraph();
                cx.stack.push(Container::Quote(Vec::new()));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                cx.flush_paragraph();
                if let Some(Container::Quote(blocks)) = cx.stack.pop() {
                    cx.push_block(MdBlock::Quote(blocks));
                }
            }
            Event::Start(Tag::List(start)) => {
                cx.flush_paragraph();
                cx.stack.push(Container::List {
                    start,
                    items: Vec::new(),
                });
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(Container::List { start, items }) = cx.stack.pop() {
                    cx.push_block(MdBlock::List { start, items });
                }
            }
            Event::Start(Tag::Item) => {
                cx.flush_paragraph();
                cx.stack.push(Container::Item(Vec::new()));
            }
            Event::End(TagEnd::Item) => {
                cx.flush_paragraph();
                if let Some(Container::Item(blocks)) = cx.stack.pop() {
                    if let Some(Container::List { items, .. }) = cx.stack.last_mut() {
                        items.push(blocks);
                    }
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                cx.flush_paragraph();
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                cx.code = Some((lang, String::new()));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, code)) = cx.code.take() {
                    cx.push_block(MdBlock::CodeBlock {
                        lang,
                        code: code.trim_end_matches('\n').to_owned(),
                    });
                }
            }
            Event::Start(Tag::Emphasis) => cx.emphasis += 1,
            Event::End(TagEnd::Emphasis) => cx.emphasis = cx.emphasis.saturating_sub(1),
            Event::Start(Tag::Strong) => cx.strong += 1,
            Event::End(TagEnd::Strong) => cx.strong = cx.strong.saturating_sub(1),
            Event::Start(Tag::Strikethrough) => cx.strike += 1,
            Event::End(TagEnd::Strikethrough) => cx.strike = cx.strike.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => {
                cx.link = Some((dest_url.to_string(), String::new()));
            }
            Event::End(TagEnd::Link) => {
                if let Some((url, text)) = cx.link.take() {
                    cx.push_span(MdSpan::Link { text, url });
                }
            }
            Event::Start(Tag::Table(_)) => {
                cx.flush_paragraph();
                cx.table = Some(TableCx::default());
            }
            Event::End(TagEnd::Table) => {
                if let Some(table) = cx.table.take() {
                    cx.push_block(MdBlock::Table {
                        header: table.header,
                        rows: table.rows,
                    });
                }
            }
            Event::Start(Tag::TableHead) => {
                if let Some(t) = &mut cx.table {
                    t.in_head = true;
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(t) = &mut cx.table {
                    t.in_head = false;
                }
            }
            Event::Start(Tag::TableRow) => {
                if let Some(t) = &mut cx.table {
                    t.row.clear();
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(t) = &mut cx.table {
                    let row = std::mem::take(&mut t.row);
                    t.rows.push(row);
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(t) = &mut cx.table {
                    t.cell.clear();
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(t) = &mut cx.table {
                    let cell = std::mem::take(&mut t.cell);
                    if t.in_head {
                        t.header.push(cell);
                    } else {
                        t.row.push(cell);
                    }
                }
            }
            Event::Rule => {
                cx.flush_paragraph();
                cx.push_block(MdBlock::Rule);
            }
            Event::Text(s) => cx.text(&s),
            Event::Code(s) => {
                if let Some((_, text)) = &mut cx.link {
                    text.push_str(&s);
                } else {
                    cx.push_span(MdSpan::Code(s.to_string()));
                }
            }
            Event::SoftBreak => cx.text(" "),
            Event::HardBreak => cx.text("\n"),
            // Raw HTML is intentionally dropped (never rendered).
            _ => {}
        }
    }
    cx.flush_paragraph();
    // Unwind any unclosed containers conservatively.
    while let Some(container) = cx.stack.pop() {
        match container {
            Container::Quote(blocks) => cx.push_block(MdBlock::Quote(blocks)),
            Container::Item(blocks) => cx.push_block(MdBlock::List {
                start: None,
                items: vec![blocks],
            }),
            Container::List { start, items } => cx.push_block(MdBlock::List { start, items }),
        }
    }
    cx.root
}

/* ---------------- rendering ---------------- */

#[derive(Clone, Copy)]
struct InlineStyle {
    size: f32,
    weight: FontWeight,
    color: egui::Color32,
    italics: bool,
}

fn render_inline(ui: &mut Ui, t: &Theme, spans: &[MdSpan], base: InlineStyle) {
    render_inline_prefixed(ui, t, None, spans, base);
}

/// Inline flow with an optional leading run (list markers) so the marker
/// shares the first row's baseline.
fn render_inline_prefixed(
    ui: &mut Ui,
    t: &Theme,
    prefix: Option<&str>,
    spans: &[MdSpan],
    base: InlineStyle,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::new(0.0, 2.0);
        if let Some(prefix) = prefix {
            ui.label(
                RichText::new(prefix)
                    .font(t.font(ui.ctx(), FontWeight::Regular, base.size))
                    .color(t.fg[2]),
            );
        }
        for span in spans {
            match span {
                MdSpan::Text {
                    text,
                    strong,
                    emphasis,
                    strike,
                } => {
                    let weight = if *strong {
                        match base.weight {
                            FontWeight::Regular => FontWeight::Medium,
                            w => w,
                        }
                    } else {
                        base.weight
                    };
                    let color = if *strong { t.fg[0] } else { base.color };
                    let mut rt = RichText::new(text.as_str())
                        .font(t.font(ui.ctx(), weight, base.size))
                        .color(color);
                    if *emphasis || base.italics {
                        rt = rt.italics();
                    }
                    if *strike {
                        rt = rt.strikethrough();
                    }
                    ui.label(rt);
                }
                MdSpan::Code(code) => {
                    ui.label(
                        RichText::new(format!(" {code} "))
                            .font(t.mono(base.size - 1.0))
                            .color(t.accent.fg)
                            .background_color(t.bg[2]),
                    );
                }
                MdSpan::Link { text, url } => {
                    let rt = RichText::new(text.as_str())
                        .font(t.font(ui.ctx(), base.weight, base.size))
                        .color(t.accent.fg);
                    match safe_url(url) {
                        Some(url) => {
                            // Underline on hover comes from egui's link visuals;
                            // click opens in the system browser.
                            let _ = ui.hyperlink_to(rt, url.to_owned());
                        }
                        // Unsafe scheme: plain text, never a hyperlink.
                        None => {
                            ui.label(
                                RichText::new(text.as_str())
                                    .font(t.font(ui.ctx(), base.weight, base.size))
                                    .color(base.color),
                            );
                        }
                    }
                }
            }
        }
    });
}

/// A table cell as one unwrapped [`egui::text::LayoutJob`] label — Grid cells
/// size correctly this way (wrapped inline flow collapses inside grids).
/// Links keep their accent color but aren't clickable here; keep basic.
fn table_cell(ui: &mut Ui, t: &Theme, spans: &[MdSpan], base: InlineStyle) {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    for span in spans {
        match span {
            MdSpan::Text {
                text,
                strong,
                emphasis,
                strike,
            } => {
                let weight = if *strong {
                    FontWeight::Medium
                } else {
                    base.weight
                };
                let mut format = TextFormat {
                    font_id: t.font(ui.ctx(), weight, base.size),
                    color: if *strong { t.fg[0] } else { base.color },
                    italics: *emphasis || base.italics,
                    ..Default::default()
                };
                if *strike {
                    format.strikethrough = egui::Stroke::new(1.0, base.color);
                }
                job.append(text, 0.0, format);
            }
            MdSpan::Code(code) => {
                job.append(
                    &format!(" {code} "),
                    0.0,
                    TextFormat {
                        font_id: t.mono(base.size - 1.0),
                        color: t.accent.fg,
                        background: t.bg[2],
                        ..Default::default()
                    },
                );
            }
            MdSpan::Link { text, .. } => {
                job.append(
                    text,
                    0.0,
                    TextFormat {
                        font_id: t.font(ui.ctx(), base.weight, base.size),
                        color: t.accent.fg,
                        ..Default::default()
                    },
                );
            }
        }
    }
    ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
}

fn code_well(ui: &mut Ui, t: &Theme, lang: &str, code: &str) {
    Frame::new()
        .fill(t.bg[1])
        .stroke(Stroke::new(1.0, t.border.subtle))
        .corner_radius(CornerRadius::same(t.radius.md as u8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 20.0);
            #[cfg(feature = "code")]
            {
                let job = crate::widgets::specialty::code::highlight_job(
                    ui,
                    t,
                    code,
                    lang,
                    t.type_scale.sm,
                );
                ui.add(egui::Label::new(job));
            }
            #[cfg(not(feature = "code"))]
            {
                let _ = lang;
                ui.label(
                    RichText::new(code)
                        .font(t.mono(t.type_scale.sm))
                        .color(t.fg[1]),
                );
            }
        });
}

fn render_blocks(ui: &mut Ui, t: &Theme, blocks: &[MdBlock], quote: bool) {
    let body = InlineStyle {
        size: t.type_scale.base,
        weight: FontWeight::Regular,
        color: if quote { t.fg[2] } else { t.fg[1] },
        italics: quote,
    };
    for (bi, block) in blocks.iter().enumerate() {
        match block {
            MdBlock::Heading { level, spans } => {
                let size = match level {
                    1 => t.type_scale.h3,
                    2 => t.type_scale.lg,
                    3 => t.type_scale.md,
                    _ => t.type_scale.base,
                };
                render_inline(
                    ui,
                    t,
                    spans,
                    InlineStyle {
                        size,
                        weight: FontWeight::SemiBold,
                        color: t.fg[0],
                        italics: false,
                    },
                );
            }
            MdBlock::Paragraph(spans) => render_inline(ui, t, spans, body),
            MdBlock::CodeBlock { lang, code } => code_well(ui, t, lang, code),
            MdBlock::Quote(blocks) => {
                let inner = ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.vertical(|ui| render_blocks(ui, t, blocks, true));
                });
                let rect = inner.response.rect;
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.min.x + 2.0, rect.min.y + 2.0),
                        egui::pos2(rect.min.x + 5.0, rect.max.y - 2.0),
                    ),
                    CornerRadius::same(1),
                    t.border.strong,
                );
            }
            MdBlock::List { start, items } => {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (i, item) in items.iter().enumerate() {
                        let marker = match start {
                            Some(s) => format!("{}. ", s + i as u64),
                            None => "•  ".to_owned(),
                        };
                        let marker_w = ui
                            .painter()
                            .layout_no_wrap(
                                marker.clone(),
                                t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                                t.fg[2],
                            )
                            .size()
                            .x;
                        // The marker joins the first paragraph's inline flow so
                        // it shares the baseline; any remaining blocks (nested
                        // lists, code, …) indent under it.
                        let (first, rest) = match item.first() {
                            Some(MdBlock::Paragraph(spans)) => (Some(spans), &item[1..]),
                            _ => (None, &item[..]),
                        };
                        match first {
                            Some(spans) => {
                                render_inline_prefixed(ui, t, Some(&marker), spans, body)
                            }
                            None => {
                                ui.label(
                                    RichText::new(&marker)
                                        .font(t.font(
                                            ui.ctx(),
                                            FontWeight::Regular,
                                            t.type_scale.base,
                                        ))
                                        .color(t.fg[2]),
                                );
                            }
                        }
                        if !rest.is_empty() {
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.add_space(marker_w);
                                ui.vertical(|ui| render_blocks(ui, t, rest, quote));
                            });
                        }
                    }
                });
            }
            MdBlock::Rule => {
                let _ = Separator::new().show(ui);
            }
            MdBlock::Table { header, rows } => {
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
                egui::Grid::new(("forge-md-table", bi))
                    .spacing(egui::Vec2::new(16.0, 4.0))
                    .show(ui, |ui| {
                        for cell in header {
                            table_cell(ui, t, cell, head_style);
                        }
                        if !header.is_empty() {
                            ui.end_row();
                        }
                        for row in rows {
                            for cell in row {
                                table_cell(ui, t, cell, cell_style);
                            }
                            ui.end_row();
                        }
                    });
            }
        }
    }
}

/// Markdown block widget: `Markdown::new(source).show(ui)`.
#[derive(Clone, Debug)]
pub struct Markdown<'a> {
    source: &'a str,
}

impl<'a> Markdown<'a> {
    pub fn new(source: &'a str) -> Markdown<'a> {
        Markdown { source }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let blocks = parse(self.source);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = t.space.x(2.0);
            render_blocks(ui, &t, &blocks, false);
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_allows_web_schemes_only() {
        assert_eq!(safe_url("https://forge.dev"), Some("https://forge.dev"));
        assert_eq!(safe_url("http://forge.dev"), Some("http://forge.dev"));
        assert_eq!(safe_url("mailto:x@y.z"), Some("mailto:x@y.z"));
        assert_eq!(safe_url("  HTTPS://x "), Some("HTTPS://x"));
        assert_eq!(safe_url("javascript:alert(1)"), None);
        assert_eq!(safe_url("JaVaScRiPt:alert(1)"), None);
        assert_eq!(safe_url("data:text/html,<x>"), None);
        assert_eq!(safe_url("vbscript:x"), None);
        assert_eq!(safe_url("/relative/path"), None);
    }

    #[test]
    fn headings_map_levels() {
        let blocks = parse("# One\n\n### Three");
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], MdBlock::Heading { level: 1, spans }
            if matches!(&spans[0], MdSpan::Text { text, .. } if text == "One")));
        assert!(matches!(&blocks[1], MdBlock::Heading { level: 3, .. }));
    }

    #[test]
    fn fenced_code_keeps_language() {
        let blocks = parse("```rust\nfn main() {}\n```");
        assert_eq!(
            blocks,
            vec![MdBlock::CodeBlock {
                lang: "rust".to_owned(),
                code: "fn main() {}".to_owned(),
            }]
        );
    }

    #[test]
    fn links_and_inline_code_are_captured() {
        let blocks = parse("see [docs](https://forge.dev) and `cargo run`");
        let MdBlock::Paragraph(spans) = &blocks[0] else {
            panic!("expected paragraph, got {blocks:?}");
        };
        assert!(spans.iter().any(|s| matches!(s, MdSpan::Link { text, url }
            if text == "docs" && url == "https://forge.dev")));
        assert!(spans
            .iter()
            .any(|s| matches!(s, MdSpan::Code(c) if c == "cargo run")));
    }

    #[test]
    fn strong_and_emphasis_flags() {
        let blocks = parse("**bold** and *lean*");
        let MdBlock::Paragraph(spans) = &blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(spans
            .iter()
            .any(|s| matches!(s, MdSpan::Text { text, strong: true, .. } if text == "bold")));
        assert!(spans
            .iter()
            .any(|s| matches!(s, MdSpan::Text { text, emphasis: true, .. } if text == "lean")));
    }

    #[test]
    fn ordered_list_start_and_items() {
        let blocks = parse("3. three\n4. four");
        let MdBlock::List { start, items } = &blocks[0] else {
            panic!("expected list, got {blocks:?}");
        };
        assert_eq!(*start, Some(3));
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn quote_nests_blocks() {
        let blocks = parse("> quoted text");
        let MdBlock::Quote(inner) = &blocks[0] else {
            panic!("expected quote, got {blocks:?}");
        };
        assert!(matches!(&inner[0], MdBlock::Paragraph(_)));
    }

    #[test]
    fn tables_split_header_and_rows() {
        let blocks = parse("| a | b |\n| - | - |\n| 1 | 2 |");
        let MdBlock::Table { header, rows } = &blocks[0] else {
            panic!("expected table, got {blocks:?}");
        };
        assert_eq!(header.len(), 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 2);
    }

    #[test]
    fn raw_html_is_dropped() {
        let blocks = parse("before\n\n<script>alert(1)</script>\n\nafter");
        for block in &blocks {
            if let MdBlock::Paragraph(spans) = block {
                for span in spans {
                    if let MdSpan::Text { text, .. } = span {
                        assert!(!text.contains("<script>"));
                    }
                }
            }
        }
    }
}
