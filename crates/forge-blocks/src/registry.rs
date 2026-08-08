//! The kind registry — one entry per [`BlockKind`] variant carrying the
//! policy each kit used to keep its own copy of: the palette label, whether
//! the kind is a data block, the payload a fresh block starts with, and the
//! markdown form [`crate::to_markdown`] writes it in.
//!
//! [`BlockKind::type_name`] is the bridge from a value to its entry, and its
//! match is exhaustive: a new schema variant does not compile until it is
//! named here, and `tests/registry.rs` fails until it has an entry.

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
    },
    KindEntry {
        type_name: "quote",
        label: "Quote",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Quote { md: String::new() },
    },
    KindEntry {
        type_name: "divider",
        label: "Divider",
        is_data: false,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Divider,
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
    },
    KindEntry {
        type_name: "columns",
        label: "Columns",
        is_data: false,
        markdown: MarkdownForm::Flattened,
        starter: || BlockKind::Columns {
            columns: vec![empty_column(), empty_column()],
        },
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
    },
    KindEntry {
        type_name: "math",
        label: "Math",
        is_data: true,
        markdown: MarkdownForm::Native,
        starter: || BlockKind::Math { tex: String::new() },
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
    },
];

/// A column holding one empty paragraph, half the width.
fn empty_column() -> Column {
    Column {
        ratio: 0.5,
        blocks: vec![Block::new(BlockKind::Paragraph { md: String::new() })],
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
