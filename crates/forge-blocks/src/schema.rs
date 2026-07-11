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
            | BlockKind::Admonition { md, .. } => Some(md),
            _ => None,
        }
    }

    pub fn md_mut(&mut self) -> Option<&mut String> {
        match self {
            BlockKind::Paragraph { md }
            | BlockKind::Heading { md, .. }
            | BlockKind::ListItem { md, .. }
            | BlockKind::Quote { md }
            | BlockKind::Admonition { md, .. } => Some(md),
            _ => None,
        }
    }

    /// Whether the kind edits as a plain text block (has an `md` body edited
    /// with the shared text keyboard model).
    pub fn is_text(&self) -> bool {
        self.md().is_some()
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
