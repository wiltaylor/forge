//! A rich sample document exercising every block kind — shared by the TUI
//! and egui gallery sections (and mirrored by the web demo) so all platforms
//! demo the same content.

use crate::schema::{Block, BlockKind, Column, Document, ListStyle, Tone};

fn b(kind: BlockKind) -> Block {
    Block::new(kind)
}

/// Every block kind, inline styles, emoji, and a two-column layout.
pub fn sample_document() -> Document {
    Document::from_blocks(vec![
        b(BlockKind::Heading {
            level: 1,
            md: "Forge Blocks :rocket:".into(),
        }),
        b(BlockKind::Paragraph {
            md: "A **block-based** page editor with *inline markdown*, `code`, \
                 [links](https://example.com), ~~regrets~~, and :sparkles: emoji. \
                 Focus a block to edit its raw source; press `/` on an empty block \
                 for the block palette."
                .into(),
        }),
        b(BlockKind::Heading {
            level: 2,
            md: "Typography".into(),
        }),
        b(BlockKind::ListItem {
            style: ListStyle::Bullet,
            checked: None,
            indent: 0,
            md: "Bullet lists with **bold** entries".into(),
        }),
        b(BlockKind::ListItem {
            style: ListStyle::Bullet,
            checked: None,
            indent: 1,
            md: "nested by indent".into(),
        }),
        b(BlockKind::ListItem {
            style: ListStyle::Number,
            checked: None,
            indent: 0,
            md: "Numbered items".into(),
        }),
        b(BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(true),
            indent: 0,
            md: "Ship the schema".into(),
        }),
        b(BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(false),
            indent: 0,
            md: "Ship the editors".into(),
        }),
        b(BlockKind::Quote {
            md: "Blocks all the way down.".into(),
        }),
        b(BlockKind::Divider),
        b(BlockKind::Heading {
            level: 2,
            md: "Code".into(),
        }),
        b(BlockKind::Code {
            lang: "rust".into(),
            code: "fn main() {\n    println!(\"hello, blocks\");\n}".into(),
        }),
        b(BlockKind::Heading {
            level: 2,
            md: "Data".into(),
        }),
        b(BlockKind::Table {
            header: vec!["Kit".into(), "Language".into(), "Status".into()],
            rows: vec![
                vec![
                    "web".into(),
                    "SolidJS".into(),
                    ":white_check_mark: shipped".into(),
                ],
                vec![
                    "tui".into(),
                    "**Rust**".into(),
                    ":white_check_mark: shipped".into(),
                ],
                vec![
                    "egui".into(),
                    "**Rust**".into(),
                    ":hourglass: rolling".into(),
                ],
            ],
        }),
        b(BlockKind::Admonition {
            tone: Tone::Warning,
            title: "Careful".into(),
            md: "Admonitions carry a tone, a title, and an **inline-markdown** body.".into(),
        }),
        b(BlockKind::Admonition {
            tone: Tone::Info,
            title: "Tip".into(),
            md: "Type `:::danger` at the start of a paragraph to convert it.".into(),
        }),
        b(BlockKind::Heading {
            level: 2,
            md: "Columns".into(),
        }),
        b(BlockKind::Columns {
            columns: vec![
                Column {
                    ratio: 0.5,
                    blocks: vec![
                        b(BlockKind::Heading {
                            level: 3,
                            md: "Left".into(),
                        }),
                        b(BlockKind::Paragraph {
                            md: "Columns split content side by side.".into(),
                        }),
                    ],
                },
                Column {
                    ratio: 0.5,
                    blocks: vec![
                        b(BlockKind::Heading {
                            level: 3,
                            md: "Right".into(),
                        }),
                        b(BlockKind::Paragraph {
                            md: "Each cell holds its own block list.".into(),
                        }),
                    ],
                },
            ],
        }),
        b(BlockKind::Custom {
            kind: "counter".into(),
            data: serde_json::json!({ "count": 3 }),
        }),
    ])
}
