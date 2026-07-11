//! The interchange contract: literal JSON per block kind. These exact shapes
//! are what `@forge/blocks` (web) produces and consumes — any change here is
//! a cross-platform format change and must land on both sides.

use forge_blocks::{Block, BlockKind, Column, Document, ListStyle, Tone};
use serde_json::json;

fn block(id: &str, kind: BlockKind) -> Block {
    Block {
        id: id.into(),
        kind,
    }
}

#[track_caller]
fn assert_shape(kind: BlockKind, expected: serde_json::Value) {
    let b = block("b1", kind);
    let mut want = expected;
    want["id"] = json!("b1");
    let got = serde_json::to_value(&b).unwrap();
    assert_eq!(got, want);
    let back: Block = serde_json::from_value(got).unwrap();
    assert_eq!(back, b);
}

#[test]
fn paragraph() {
    assert_shape(
        BlockKind::Paragraph {
            md: "hi **x**".into(),
        },
        json!({ "type": "paragraph", "md": "hi **x**" }),
    );
}

#[test]
fn heading() {
    assert_shape(
        BlockKind::Heading {
            level: 2,
            md: "T".into(),
        },
        json!({ "type": "heading", "level": 2, "md": "T" }),
    );
}

#[test]
fn list_item() {
    assert_shape(
        BlockKind::ListItem {
            style: ListStyle::Todo,
            checked: Some(true),
            indent: 1,
            md: "x".into(),
        },
        json!({ "type": "list_item", "style": "todo", "checked": true, "indent": 1, "md": "x" }),
    );
    // `checked` is omitted (not null) for plain bullets.
    let b = block(
        "b1",
        BlockKind::ListItem {
            style: ListStyle::Bullet,
            checked: None,
            indent: 0,
            md: "x".into(),
        },
    );
    let v = serde_json::to_value(&b).unwrap();
    assert!(v.get("checked").is_none());
    assert_eq!(v["style"], "bullet");
}

#[test]
fn quote_divider() {
    assert_shape(
        BlockKind::Quote { md: "q".into() },
        json!({ "type": "quote", "md": "q" }),
    );
    assert_shape(BlockKind::Divider, json!({ "type": "divider" }));
}

#[test]
fn code() {
    assert_shape(
        BlockKind::Code {
            lang: "rust".into(),
            code: "fn main() {}".into(),
        },
        json!({ "type": "code", "lang": "rust", "code": "fn main() {}" }),
    );
}

#[test]
fn table() {
    assert_shape(
        BlockKind::Table {
            header: vec!["A".into(), "B".into()],
            rows: vec![vec!["1".into(), "**2**".into()]],
        },
        json!({ "type": "table", "header": ["A", "B"], "rows": [["1", "**2**"]] }),
    );
}

#[test]
fn admonition() {
    assert_shape(
        BlockKind::Admonition {
            tone: Tone::Warning,
            title: "Heads up".into(),
            md: "body".into(),
        },
        json!({ "type": "admonition", "tone": "warning", "title": "Heads up", "md": "body" }),
    );
}

#[test]
fn columns() {
    assert_shape(
        BlockKind::Columns {
            columns: vec![Column {
                ratio: 0.5,
                blocks: vec![block(
                    "c1",
                    BlockKind::Paragraph {
                        md: "in col".into(),
                    },
                )],
            }],
        },
        json!({
            "type": "columns",
            "columns": [{ "ratio": 0.5, "blocks": [{ "id": "c1", "type": "paragraph", "md": "in col" }] }]
        }),
    );
}

#[test]
fn custom() {
    assert_shape(
        BlockKind::Custom {
            kind: "counter".into(),
            data: json!({ "count": 3 }),
        },
        json!({ "type": "custom", "kind": "counter", "data": { "count": 3 } }),
    );
}

#[test]
fn document_roundtrip() {
    let doc = forge_blocks::sample::sample_document();
    let text = serde_json::to_string(&doc).unwrap();
    let back: Document = serde_json::from_str(&text).unwrap();
    assert_eq!(back, doc);
    assert_eq!(back.version, forge_blocks::DOCUMENT_VERSION);
}

#[test]
fn web_fixture_parses() {
    // A document as the web editor writes it, verbatim.
    let text = r##"{
      "version": 1,
      "blocks": [
        { "id": "a", "type": "heading", "level": 1, "md": "Hello" },
        { "id": "b", "type": "paragraph", "md": "Some **bold** :rocket:" },
        { "id": "c", "type": "list_item", "style": "todo", "checked": false, "indent": 0, "md": "do it" },
        { "id": "d", "type": "divider" },
        { "id": "e", "type": "code", "lang": "ts", "code": "const x = 1;" },
        { "id": "f", "type": "table", "header": ["H"], "rows": [["c"]] },
        { "id": "g", "type": "admonition", "tone": "danger", "title": "", "md": "careful" },
        { "id": "h", "type": "columns", "columns": [
          { "ratio": 0.7, "blocks": [{ "id": "h1", "type": "paragraph", "md": "left" }] },
          { "ratio": 0.3, "blocks": [{ "id": "h2", "type": "paragraph", "md": "right" }] }
        ] },
        { "id": "i", "type": "custom", "kind": "stat", "data": { "label": "Requests", "value": "1.2k" } }
      ]
    }"##;
    let doc: Document = serde_json::from_str(text).unwrap();
    assert_eq!(doc.blocks.len(), 9);
    assert!(matches!(doc.blocks[8].kind, BlockKind::Custom { .. }));
}
