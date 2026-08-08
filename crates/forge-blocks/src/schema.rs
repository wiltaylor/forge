//! The block document schema — serde output is the frozen JSON interchange
//! shared with `@forge/blocks` (web). Every shape change must land in both.

use serde::{Deserialize, Serialize};

use crate::id::new_id;

/// Current interchange version, stored in [`Document::version`].
pub const DOCUMENT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub blocks: Vec<Block>,
}

impl Document {
    /// An empty document holding a single empty paragraph (the editor
    /// invariant: a document is never blockless).
    pub fn new() -> Self {
        Self {
            version: DOCUMENT_VERSION,
            blocks: vec![Block::new(BlockKind::Paragraph { md: String::new() })],
        }
    }

    /// Build a document from blocks; an empty list gets one empty paragraph.
    pub fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut doc = Self {
            version: DOCUMENT_VERSION,
            blocks,
        };
        doc.normalize();
        doc
    }

    /// Restore editor invariants: never blockless, and columns hold no nested
    /// columns and no empty cells (empty cells get an empty paragraph).
    pub fn normalize(&mut self) {
        for block in &mut self.blocks {
            if let BlockKind::Columns { columns } = &mut block.kind {
                for col in columns.iter_mut() {
                    col.blocks
                        .retain(|b| !matches!(b.kind, BlockKind::Columns { .. }));
                    if col.blocks.is_empty() {
                        col.blocks
                            .push(Block::new(BlockKind::Paragraph { md: String::new() }));
                    }
                }
            }
        }
        if self.blocks.is_empty() {
            self.blocks
                .push(Block::new(BlockKind::Paragraph { md: String::new() }));
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    #[serde(flatten)]
    pub kind: BlockKind,
}

impl Block {
    pub fn new(kind: BlockKind) -> Self {
        Self { id: new_id(), kind }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockKind {
    Paragraph {
        md: String,
    },
    Heading {
        level: u8,
        md: String,
    },
    ListItem {
        style: ListStyle,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        checked: Option<bool>,
        indent: u8,
        md: String,
    },
    Quote {
        md: String,
    },
    Divider,
    Code {
        lang: String,
        code: String,
    },
    /// Cells are inline-markdown strings.
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Admonition {
        tone: Tone,
        title: String,
        md: String,
    },
    /// One level only — column cells never contain another `Columns`.
    Columns {
        columns: Vec<Column>,
    },
    /// Consumer-defined block; `kind` selects the registered implementation.
    Custom {
        kind: String,
        data: serde_json::Value,
    },
    Image {
        src: String,
        alt: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<f64>,
    },
    /// `src` is a local/remote file path or a YouTube/Vimeo URL.
    Video {
        src: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        poster: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<f64>,
    },
    /// LaTeX source; renderers typeset it if they can, else show the source.
    Math {
        tex: String,
    },
    BarChart {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y_label: Option<String>,
        categories: Vec<String>,
        series: Vec<ChartSeries>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y_min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y_max: Option<f64>,
    },
    LineChart {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        x_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y_label: Option<String>,
        categories: Vec<String>,
        series: Vec<ChartSeries>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y_min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y_max: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        points: Option<Vec<ChartPoint>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        point_labels: Option<bool>,
    },
    PieChart {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        slices: Vec<PieSlice>,
    },
    /// Auto-laid-out flowchart graph.
    Diagram {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        direction: Option<DiagramDirection>,
        nodes: Vec<DiagramNode>,
        edges: Vec<DiagramEdge>,
    },
    SequenceDiagram {
        participants: Vec<SeqParticipant>,
        messages: Vec<SeqMessage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<Vec<SeqNote>>,
    },
    StateDiagram {
        states: Vec<StateNode>,
        transitions: Vec<StateTransition>,
    },
    /// DB/class-diagram style row table; an empty `title` means headerless.
    NodeTable {
        title: String,
        rows: Vec<NodeTableRow>,
    },
    Tree {
        nodes: Vec<TreeNode>,
    },
    Timeline {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        direction: Option<TimelineDirection>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phases: Option<Vec<TimelinePhase>>,
        items: Vec<TimelineItem>,
    },
    ChapterHeader {
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kicker: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reading_time: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        updated: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
    /// Footnote definition; inline `[^label]` references link to it.
    /// (`label`, not `id` — `Block.id` is flattened beside these fields.)
    Footnote {
        label: String,
        md: String,
    },
}

impl BlockKind {
    /// The inline-markdown source of text-bearing kinds (paragraph, heading,
    /// list item, quote, admonition body).
    pub fn md(&self) -> Option<&str> {
        match self {
            BlockKind::Paragraph { md }
            | BlockKind::Heading { md, .. }
            | BlockKind::ListItem { md, .. }
            | BlockKind::Quote { md }
            | BlockKind::Admonition { md, .. }
            | BlockKind::Footnote { md, .. } => Some(md),
            _ => None,
        }
    }

    pub fn md_mut(&mut self) -> Option<&mut String> {
        match self {
            BlockKind::Paragraph { md }
            | BlockKind::Heading { md, .. }
            | BlockKind::ListItem { md, .. }
            | BlockKind::Quote { md }
            | BlockKind::Admonition { md, .. }
            | BlockKind::Footnote { md, .. } => Some(md),
            _ => None,
        }
    }

    /// Whether the kind edits as a plain text block (has an `md` body edited
    /// with the shared text keyboard model).
    pub fn is_text(&self) -> bool {
        self.md().is_some()
    }

    /// Whether the kind is a data block: rendered from structured fields and
    /// edited as raw JSON source rather than through a bespoke editor.
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            BlockKind::Image { .. }
                | BlockKind::Video { .. }
                | BlockKind::Math { .. }
                | BlockKind::BarChart { .. }
                | BlockKind::LineChart { .. }
                | BlockKind::PieChart { .. }
                | BlockKind::Diagram { .. }
                | BlockKind::SequenceDiagram { .. }
                | BlockKind::StateDiagram { .. }
                | BlockKind::NodeTable { .. }
                | BlockKind::Tree { .. }
                | BlockKind::Timeline { .. }
                | BlockKind::ChapterHeader { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListStyle {
    Bullet,
    Number,
    Todo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Info,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub ratio: f32,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
}

/// A labelled point annotation on a line chart; `category` indexes into the
/// chart's `categories`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartPoint {
    pub label: String,
    pub category: u32,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramDirection {
    Right,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramNodeKind {
    Process,
    Decision,
    Terminator,
    Node,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub kind: DiagramNodeKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramEdgeKind {
    Solid,
    Dashed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiagramEdge {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<DiagramEdgeKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParticipantKind {
    Box,
    Actor,
    External,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeqParticipant {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ParticipantKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Sync,
    Async,
    Reply,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeqMessage {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MessageKind>,
}

/// A note anchored under message index `at`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeqNote {
    pub at: u32,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateNode {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<bool>,
    #[serde(rename = "final", default, skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
}

/// `key` is the row's stable identifier (wdoc uses it as an edge target).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeTableRow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub md: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelineDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelineSide {
    Near,
    Far,
}

/// `from`/`to` are ISO-8601 date strings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelinePhase {
    pub label: String,
    pub from: String,
    pub to: String,
}

/// `on` is an ISO-8601 date string.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineItem {
    pub label: String,
    pub on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<TimelineSide>,
}
