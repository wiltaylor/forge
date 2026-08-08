//! The kind registry — one entry per [`BlockKind`] variant carrying the
//! policy each kit used to keep its own copy of: the palette label, whether
//! the kind is a data block, the payload a fresh block starts with, the
//! markdown form [`crate::to_markdown`] writes it in, the wire fields the
//! kind carries, and the slash-palette rows that make one.
//!
//! [`BlockKind::type_name`] is the bridge from a value to its entry, and its
//! match is exhaustive: a new schema variant does not compile until it is
//! named here, and `tests/registry.rs` fails until it has an entry.
//!
//! The Rust kits read this module directly. The web kit cannot, so
//! [`crate::export`] dumps it to `contract/blocks-registry.json` and the Node
//! generators under `scripts/generate/` emit the TypeScript from that.

use crate::schema::{
    Block, BlockKind, ChartSeries, Column, DiagramEdge, DiagramNode, DiagramNodeKind, ListStyle,
    MessageKind, NodeTableRow, PieSlice, SeqMessage, SeqParticipant, StateNode, StateTransition,
    TimelineItem, Tone, TreeNode,
};

/// How [`crate::to_markdown`] writes a kind. Markdown has a spelling for the
/// text kinds; everything else travels as a fence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownForm {
    /// Markdown's own syntax — a heading writes hashes, a list item writes its
    /// bullet or its number.
    Native,
    /// Markdown's own syntax where the fields allow, else [`MarkdownForm::Fence`]:
    /// a sized image and a multi-line footnote have no markdown spelling.
    NativeOrFence,
    /// A ```` ```forge:<type> ```` fence holding the block's JSON fields.
    Fence,
    /// A ```` ```block:<kind> ```` fence holding the consumer's JSON payload.
    CustomFence,
    /// No form of its own — the children flatten into the surrounding blocks,
    /// the one form markdown cannot carry back.
    Flattened,
}

/// One wire field of a kind, in serialisation order.
///
/// `ts` is the field's TypeScript spelling, because the web kit's union is
/// generated from these entries and TypeScript is the only language that
/// needs the shape written out — Rust reads it off [`BlockKind`] itself. The
/// names it uses (`ChartSeries`, `TimelineItem`, …) are the hand-written
/// helper types in `packages/blocks/src/wire.ts`, which mirror the schema
/// structs of the same name.
#[derive(Clone, Copy, Debug)]
pub struct WireField {
    /// The JSON key.
    pub name: &'static str,
    /// The TypeScript type of the value.
    pub ts: &'static str,
    /// Whether serde omits the key when the field is unset.
    pub optional: bool,
}

/// A required field.
const fn req(name: &'static str, ts: &'static str) -> WireField {
    WireField {
        name,
        ts,
        optional: false,
    }
}

/// A field serde omits when it is `None`.
const fn opt(name: &'static str, ts: &'static str) -> WireField {
    WireField {
        name,
        ts,
        optional: true,
    }
}

/// What picking a slash-palette row does.
#[derive(Clone, Copy, Debug)]
pub enum PaletteAction {
    /// Insert a fresh block of this kind, or convert the typed-into block to
    /// it. One kind can offer several — a heading offers one row per level.
    Insert(fn() -> BlockKind),
    /// Wrap the block into `n` columns rather than replace it.
    WrapColumns(u8),
}

/// One row of the slash palette.
#[derive(Clone, Copy, Debug)]
pub struct PaletteRow {
    /// Stable identifier, used by the kits that filter on a command id rather
    /// than on the label.
    pub id: &'static str,
    /// What the row reads as.
    pub label: &'static str,
    /// The markdown shortcut that produces the same block, shown beside the
    /// label by the kits that have room for it.
    pub hint: Option<&'static str>,
    /// What picking the row does.
    pub action: PaletteAction,
}

/// A row that inserts a kind.
const fn insert(
    id: &'static str,
    label: &'static str,
    hint: Option<&'static str>,
    make: fn() -> BlockKind,
) -> PaletteRow {
    PaletteRow {
        id,
        label,
        hint,
        action: PaletteAction::Insert(make),
    }
}

/// What the registry knows about one block kind.
#[derive(Clone, Copy, Debug)]
pub struct KindEntry {
    /// The wire `type` discriminant — the serde tag, and the registry key.
    pub type_name: &'static str,
    /// What a palette calls this kind. The kits still carry their own tables;
    /// the labels here are the ones they agree on, plus a name for the kinds
    /// no palette offers as a single row.
    pub label: &'static str,
    /// Whether the kind is a data block: rendered from structured fields and
    /// edited as raw JSON. Mirrors [`BlockKind::is_data`], which stays the
    /// predicate the editing path routes on.
    pub is_data: bool,
    /// The markdown form the converter writes.
    pub markdown: MarkdownForm,
    /// The payload a freshly inserted block of this kind carries.
    pub starter: fn() -> BlockKind,
    /// Prose about the kind, one entry per line, copied into the generated
    /// union as a doc comment.
    pub doc: &'static [&'static str],
    /// The fields the kind serialises, in wire order and beside the `type`
    /// tag. [`fields_are_exhaustive`] fails to compile when a variant gains
    /// or loses one.
    pub fields: &'static [WireField],
    /// The slash-palette rows this kind contributes, in palette order. Empty
    /// for a kind no palette offers.
    pub palette: &'static [PaletteRow],
}

/// Every kind the schema defines, in declaration order. A `static` so that
/// one entry has one address, whoever reads it.
pub static KINDS: &[KindEntry] = &[
    KindEntry {
        type_name: "paragraph",
        label: "Text",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Paragraph { md: String::new() },
        doc: &[],
        fields: &[req("md", "string")],
        palette: &[insert("text", "Text", None, || starter_of("paragraph"))],
    },
    KindEntry {
        type_name: "heading",
        label: "Heading",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Heading {
            level: 1,
            md: String::new(),
        },
        doc: &[],
        fields: &[req("level", "1 | 2 | 3 | 4"), req("md", "string")],
        // Four levels — the set two of the three kits already offer.
        palette: &[
            insert("h1", "Heading 1", Some("#"), || heading(1)),
            insert("h2", "Heading 2", Some("##"), || heading(2)),
            insert("h3", "Heading 3", Some("###"), || heading(3)),
            insert("h4", "Heading 4", Some("####"), || heading(4)),
        ],
    },
    KindEntry {
        type_name: "list_item",
        label: "List item",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::ListItem {
            style: ListStyle::Bullet,
            checked: None,
            indent: 0,
            md: String::new(),
        },
        doc: &[],
        fields: &[
            req("style", "ListStyle"),
            opt("checked", "boolean"),
            req("indent", "number"),
            req("md", "string"),
        ],
        palette: &[
            insert("bullet", "Bullet list", Some("-"), || {
                list_item(ListStyle::Bullet, None)
            }),
            insert("number", "Numbered list", Some("1."), || {
                list_item(ListStyle::Number, None)
            }),
            insert("todo", "To-do list", Some("[]"), || {
                list_item(ListStyle::Todo, Some(false))
            }),
        ],
    },
    KindEntry {
        type_name: "quote",
        label: "Quote",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Quote { md: String::new() },
        doc: &[],
        fields: &[req("md", "string")],
        palette: &[insert("quote", "Quote", Some(">"), || starter_of("quote"))],
    },
    KindEntry {
        type_name: "divider",
        label: "Divider",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Divider,
        doc: &[],
        fields: &[],
        palette: &[insert("divider", "Divider", Some("---"), || {
            starter_of("divider")
        })],
    },
    KindEntry {
        type_name: "code",
        label: "Code",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Code {
            lang: String::new(),
            code: String::new(),
        },
        doc: &[],
        fields: &[req("lang", "string"), req("code", "string")],
        palette: &[insert("code", "Code", Some("```"), || starter_of("code"))],
    },
    KindEntry {
        type_name: "table",
        label: "Table",
        is_data: false,
        markdown: MarkdownForm::Native,
        // Three wide and two rows deep — the shape two of the three kits
        // already start with.
        starter: || BlockKind::Table {
            header: vec![String::new(); 3],
            rows: vec![vec![String::new(); 3]; 2],
        },
        doc: &["Cells are inline-markdown strings."],
        fields: &[req("header", "string[]"), req("rows", "string[][]")],
        palette: &[insert("table", "Table", None, || starter_of("table"))],
    },
    KindEntry {
        type_name: "admonition",
        label: "Callout",
        is_data: false,
        // The GitHub alert form: a blockquote led by `[!TONE]`.
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Admonition {
            tone: Tone::Info,
            title: String::new(),
            md: String::new(),
        },
        doc: &[],
        fields: &[
            req("tone", "AdmonitionTone"),
            req("title", "string"),
            req("md", "string"),
        ],
        palette: &[insert("callout", "Callout", Some(":::"), || {
            starter_of("admonition")
        })],
    },
    KindEntry {
        type_name: "columns",
        label: "Columns",
        is_data: false,
        markdown: MarkdownForm::Flattened,
        starter: || BlockKind::Columns {
            columns: vec![empty_column(), empty_column()],
        },
        doc: &["One level only — column cells never contain another `columns` block."],
        fields: &[req("columns", "BlockColumn[]")],
        // The two rows that wrap rather than replace. They are commands, not
        // kinds, so `palette_rows` puts them last.
        palette: &[
            PaletteRow {
                id: "col2",
                label: "2 columns",
                hint: None,
                action: PaletteAction::WrapColumns(2),
            },
            PaletteRow {
                id: "col3",
                label: "3 columns",
                hint: None,
                action: PaletteAction::WrapColumns(3),
            },
        ],
    },
    KindEntry {
        type_name: "custom",
        label: "Custom",
        is_data: false,
        markdown: MarkdownForm::CustomFence,
        starter: || BlockKind::Custom {
            kind: String::new(),
            data: serde_json::Value::Null,
        },
        doc: &["Consumer-defined block; `kind` selects the implementation the host registered."],
        fields: &[req("kind", "string"), req("data", "unknown")],
        // No row of its own: a kit lists one row per registered custom kind.
        palette: &[],
    },
    KindEntry {
        type_name: "image",
        label: "Image",
        is_data: true,
        markdown: MarkdownForm::NativeOrFence,
        starter: || BlockKind::Image {
            src: String::new(),
            alt: String::new(),
            width: None,
            height: None,
        },
        doc: &[],
        fields: &[
            req("src", "string"),
            req("alt", "string"),
            opt("width", "number"),
            opt("height", "number"),
        ],
        palette: &[insert("image", "Image", Some("![]"), || {
            starter_of("image")
        })],
    },
    KindEntry {
        type_name: "video",
        label: "Video",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::Video {
            src: String::new(),
            poster: None,
            title: None,
            width: None,
            height: None,
        },
        doc: &["`src` is a local/remote file path or a YouTube/Vimeo URL."],
        fields: &[
            req("src", "string"),
            opt("poster", "string"),
            opt("title", "string"),
            opt("width", "number"),
            opt("height", "number"),
        ],
        palette: &[insert("video", "Video", Some("embed"), || {
            starter_of("video")
        })],
    },
    KindEntry {
        type_name: "math",
        label: "Math",
        is_data: true,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Math { tex: String::new() },
        doc: &["LaTeX source; renderers typeset it if they can, else show the source."],
        fields: &[req("tex", "string")],
        palette: &[insert("math", "Math", Some("$$"), || starter_of("math"))],
    },
    KindEntry {
        type_name: "bar_chart",
        label: "Bar chart",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::BarChart {
            title: None,
            x_label: None,
            y_label: None,
            categories: vec!["A".into(), "B".into(), "C".into()],
            series: vec![ChartSeries {
                name: "Series 1".into(),
                values: vec![3.0, 5.0, 4.0],
            }],
            y_min: None,
            y_max: None,
        },
        doc: &[],
        fields: &[
            opt("title", "string"),
            opt("x_label", "string"),
            opt("y_label", "string"),
            req("categories", "string[]"),
            req("series", "ChartSeries[]"),
            opt("y_min", "number"),
            opt("y_max", "number"),
        ],
        palette: &[insert("bar_chart", "Bar chart", None, || {
            starter_of("bar_chart")
        })],
    },
    KindEntry {
        type_name: "line_chart",
        label: "Line chart",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::LineChart {
            title: None,
            x_label: None,
            y_label: None,
            categories: vec!["A".into(), "B".into(), "C".into()],
            series: vec![ChartSeries {
                name: "Series 1".into(),
                values: vec![3.0, 5.0, 4.0],
            }],
            y_min: None,
            y_max: None,
            points: None,
            point_labels: None,
        },
        doc: &[],
        fields: &[
            opt("title", "string"),
            opt("x_label", "string"),
            opt("y_label", "string"),
            req("categories", "string[]"),
            req("series", "ChartSeries[]"),
            opt("y_min", "number"),
            opt("y_max", "number"),
            opt("points", "ChartPoint[]"),
            opt("point_labels", "boolean"),
        ],
        palette: &[insert("line_chart", "Line chart", None, || {
            starter_of("line_chart")
        })],
    },
    KindEntry {
        type_name: "pie_chart",
        label: "Pie chart",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::PieChart {
            title: None,
            slices: vec![
                PieSlice {
                    label: "A".into(),
                    value: 3.0,
                },
                PieSlice {
                    label: "B".into(),
                    value: 5.0,
                },
            ],
        },
        doc: &[],
        fields: &[opt("title", "string"), req("slices", "PieSlice[]")],
        palette: &[insert("pie_chart", "Pie chart", None, || {
            starter_of("pie_chart")
        })],
    },
    KindEntry {
        type_name: "diagram",
        label: "Diagram",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::Diagram {
            direction: None,
            nodes: vec![
                DiagramNode {
                    id: "start".into(),
                    kind: DiagramNodeKind::Terminator,
                    text: "Start".into(),
                },
                DiagramNode {
                    id: "work".into(),
                    kind: DiagramNodeKind::Process,
                    text: "Work".into(),
                },
                DiagramNode {
                    id: "done".into(),
                    kind: DiagramNodeKind::Terminator,
                    text: "Done".into(),
                },
            ],
            edges: vec![
                DiagramEdge {
                    from: "start".into(),
                    to: "work".into(),
                    label: None,
                    kind: None,
                },
                DiagramEdge {
                    from: "work".into(),
                    to: "done".into(),
                    label: None,
                    kind: None,
                },
            ],
        },
        doc: &["Auto-laid-out flowchart graph."],
        fields: &[
            opt("direction", "DiagramDirection"),
            req("nodes", "DiagramNode[]"),
            req("edges", "DiagramEdge[]"),
        ],
        palette: &[insert("diagram", "Diagram", Some("flow"), || {
            starter_of("diagram")
        })],
    },
    KindEntry {
        type_name: "sequence_diagram",
        label: "Sequence diagram",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::SequenceDiagram {
            participants: vec![
                SeqParticipant {
                    id: "a".into(),
                    name: Some("Client".into()),
                    kind: None,
                },
                SeqParticipant {
                    id: "b".into(),
                    name: Some("Server".into()),
                    kind: None,
                },
            ],
            messages: vec![
                SeqMessage {
                    from: "a".into(),
                    to: "b".into(),
                    text: Some("request".into()),
                    kind: None,
                },
                SeqMessage {
                    from: "b".into(),
                    to: "a".into(),
                    text: Some("response".into()),
                    kind: Some(MessageKind::Reply),
                },
            ],
            notes: None,
        },
        doc: &[],
        fields: &[
            req("participants", "SeqParticipant[]"),
            req("messages", "SeqMessage[]"),
            opt("notes", "SeqNote[]"),
        ],
        palette: &[insert("sequence_diagram", "Sequence diagram", None, || {
            starter_of("sequence_diagram")
        })],
    },
    KindEntry {
        type_name: "state_diagram",
        label: "State diagram",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::StateDiagram {
            states: vec![
                StateNode {
                    id: "idle".into(),
                    name: Some("Idle".into()),
                    initial: Some(true),
                    is_final: None,
                },
                StateNode {
                    id: "done".into(),
                    name: Some("Done".into()),
                    initial: None,
                    is_final: Some(true),
                },
            ],
            transitions: vec![StateTransition {
                from: "idle".into(),
                to: "done".into(),
                trigger: Some("finish".into()),
                guard: None,
            }],
        },
        doc: &[],
        fields: &[
            req("states", "StateNode[]"),
            req("transitions", "StateTransition[]"),
        ],
        palette: &[insert("state_diagram", "State diagram", None, || {
            starter_of("state_diagram")
        })],
    },
    KindEntry {
        type_name: "node_table",
        label: "Node table",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::NodeTable {
            title: "Table".into(),
            rows: vec![NodeTableRow {
                key: None,
                md: "row".into(),
            }],
        },
        doc: &["DB/class-diagram style row table; an empty `title` means headerless."],
        fields: &[req("title", "string"), req("rows", "NodeTableRow[]")],
        palette: &[insert("node_table", "Node table", None, || {
            starter_of("node_table")
        })],
    },
    KindEntry {
        type_name: "tree",
        label: "Tree",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::Tree {
            nodes: vec![TreeNode {
                title: "root".into(),
                icon: None,
                children: Some(vec![TreeNode {
                    title: "child".into(),
                    icon: None,
                    children: None,
                }]),
            }],
        },
        doc: &[],
        fields: &[req("nodes", "TreeNode[]")],
        palette: &[insert("tree", "Tree", None, || starter_of("tree"))],
    },
    KindEntry {
        type_name: "timeline",
        label: "Timeline",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::Timeline {
            title: None,
            direction: None,
            phases: None,
            items: vec![TimelineItem {
                label: "Start".into(),
                on: "2026-01-01".into(),
                side: None,
            }],
        },
        doc: &[],
        fields: &[
            opt("title", "string"),
            opt("direction", "TimelineDirection"),
            opt("phases", "TimelinePhase[]"),
            req("items", "TimelineItem[]"),
        ],
        palette: &[insert("timeline", "Timeline", None, || {
            starter_of("timeline")
        })],
    },
    KindEntry {
        type_name: "chapter_header",
        label: "Chapter header",
        is_data: true,
        markdown: MarkdownForm::Fence,
        starter: || BlockKind::ChapterHeader {
            title: "Title".into(),
            kicker: None,
            reading_time: None,
            updated: None,
            version: None,
        },
        doc: &[],
        fields: &[
            req("title", "string"),
            opt("kicker", "string"),
            opt("reading_time", "string"),
            opt("updated", "string"),
            opt("version", "string"),
        ],
        palette: &[insert("chapter_header", "Chapter header", None, || {
            starter_of("chapter_header")
        })],
    },
    KindEntry {
        type_name: "footnote",
        label: "Footnote",
        is_data: false,
        markdown: MarkdownForm::NativeOrFence,
        starter: || BlockKind::Footnote {
            label: "note-1".into(),
            md: String::new(),
        },
        doc: &[
            "Footnote definition; inline `[^label]` references link to it.",
            "(`label`, not `id` — the block's `id` sits beside these fields.)",
        ],
        fields: &[req("label", "string"), req("md", "string")],
        palette: &[insert("footnote", "Footnote", Some("[^]"), || {
            starter_of("footnote")
        })],
    },
];

/// A column holding one empty paragraph, half the width.
fn empty_column() -> Column {
    Column {
        ratio: 0.5,
        blocks: vec![Block::new(BlockKind::Paragraph { md: String::new() })],
    }
}

/// An empty heading at `level`.
fn heading(level: u8) -> BlockKind {
    BlockKind::Heading {
        level,
        md: String::new(),
    }
}

/// An empty list item in `style`.
fn list_item(style: ListStyle, checked: Option<bool>) -> BlockKind {
    BlockKind::ListItem {
        style,
        checked,
        indent: 0,
        md: String::new(),
    }
}

/// The registered starter for a kind — what a palette row inserts when it
/// offers the kind whole rather than one variant of it.
fn starter_of(type_name: &str) -> BlockKind {
    (kind_entry(type_name)
        .expect("a palette row names a registered kind")
        .starter)()
}

/// Every palette row, in the order the kits list them: the insert rows in
/// kind order, then the wrap actions, which are commands rather than kinds
/// and sit at the end of a palette.
pub fn palette_rows() -> Vec<&'static PaletteRow> {
    let rows = || KINDS.iter().flat_map(|entry| entry.palette.iter());
    let is_insert = |row: &&PaletteRow| matches!(row.action, PaletteAction::Insert(_));
    rows()
        .filter(is_insert)
        .chain(rows().filter(|row| !is_insert(row)))
        .collect()
}

/// Compile-time proof that [`KindEntry::fields`] is asked to keep up with the
/// schema. Every arm destructures its variant with no `..` rest pattern, so a
/// field added to or removed from [`BlockKind`] fails to compile here (E0027)
/// rather than leaving the generated TypeScript union quietly wrong.
///
/// It is a check, not a conversion: what each field *is* still lives in the
/// entry above, and `tests/registry.rs` pins the two together as far as a
/// starter payload can.
#[allow(dead_code, unused_variables)]
fn fields_are_exhaustive(kind: &BlockKind) {
    match kind {
        BlockKind::Paragraph { md } => {}
        BlockKind::Heading { level, md } => {}
        BlockKind::ListItem {
            style,
            checked,
            indent,
            md,
        } => {}
        BlockKind::Quote { md } => {}
        BlockKind::Divider => {}
        BlockKind::Code { lang, code } => {}
        BlockKind::Table { header, rows } => {}
        BlockKind::Admonition { tone, title, md } => {}
        BlockKind::Columns { columns } => {}
        BlockKind::Custom { kind, data } => {}
        BlockKind::Image {
            src,
            alt,
            width,
            height,
        } => {}
        BlockKind::Video {
            src,
            poster,
            title,
            width,
            height,
        } => {}
        BlockKind::Math { tex } => {}
        BlockKind::BarChart {
            title,
            x_label,
            y_label,
            categories,
            series,
            y_min,
            y_max,
        } => {}
        BlockKind::LineChart {
            title,
            x_label,
            y_label,
            categories,
            series,
            y_min,
            y_max,
            points,
            point_labels,
        } => {}
        BlockKind::PieChart { title, slices } => {}
        BlockKind::Diagram {
            direction,
            nodes,
            edges,
        } => {}
        BlockKind::SequenceDiagram {
            participants,
            messages,
            notes,
        } => {}
        BlockKind::StateDiagram {
            states,
            transitions,
        } => {}
        BlockKind::NodeTable { title, rows } => {}
        BlockKind::Tree { nodes } => {}
        BlockKind::Timeline {
            title,
            direction,
            phases,
            items,
        } => {}
        BlockKind::ChapterHeader {
            title,
            kicker,
            reading_time,
            updated,
            version,
        } => {}
        BlockKind::Footnote { label, md } => {}
    }
}

/// The registry entry for a wire `type` name, if the schema defines it.
pub fn kind_entry(type_name: &str) -> Option<&'static KindEntry> {
    KINDS.iter().find(|entry| entry.type_name == type_name)
}

/// Starter payload for a builtin data-block type, keyed by its wire `type`
/// name. Every kit's insert palette uses these so a freshly inserted block
/// renders meaningful content immediately; `@forge/blocks` `createBlock`
/// mirrors the same shapes byte for byte.
///
/// The text kinds are absent: a kit builds those from its own palette command
/// (a heading level, a list style), while these payloads are shared verbatim.
pub fn starter_kind(type_name: &str) -> Option<BlockKind> {
    let entry = kind_entry(type_name)?;
    (entry.is_data || entry.type_name == "footnote").then(|| (entry.starter)())
}

impl BlockKind {
    /// The wire `type` discriminant of this kind — the serde tag, and the key
    /// into the registry.
    pub fn type_name(&self) -> &'static str {
        match self {
            BlockKind::Paragraph { .. } => "paragraph",
            BlockKind::Heading { .. } => "heading",
            BlockKind::ListItem { .. } => "list_item",
            BlockKind::Quote { .. } => "quote",
            BlockKind::Divider => "divider",
            BlockKind::Code { .. } => "code",
            BlockKind::Table { .. } => "table",
            BlockKind::Admonition { .. } => "admonition",
            BlockKind::Columns { .. } => "columns",
            BlockKind::Custom { .. } => "custom",
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
            BlockKind::Footnote { .. } => "footnote",
        }
    }

    /// This kind's registry entry.
    pub fn entry(&self) -> &'static KindEntry {
        kind_entry(self.type_name()).expect("every schema variant is registered")
    }
}
