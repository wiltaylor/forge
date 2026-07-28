//! The interchange contract: literal JSON per block kind. These exact shapes
//! are what `@forge/blocks` (web) produces and consumes — any change here is
//! a cross-platform format change and must land on both sides.

use forge_blocks::{
    Block, BlockKind, ChartPoint, ChartSeries, Column, DiagramDirection, DiagramEdge,
    DiagramEdgeKind, DiagramNode, DiagramNodeKind, Document, ListStyle, MessageKind, NodeTableRow,
    ParticipantKind, PieSlice, SeqMessage, SeqNote, SeqParticipant, StateNode, StateTransition,
    TimelineDirection, TimelineItem, TimelinePhase, TimelineSide, Tone, TreeNode,
};
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
fn image() {
    assert_shape(
        BlockKind::Image {
            src: "a.png".into(),
            alt: "A".into(),
            width: Some(640.0),
            height: Some(360.0),
        },
        json!({ "type": "image", "src": "a.png", "alt": "A", "width": 640.0, "height": 360.0 }),
    );
    // Dimensions are omitted (not null) when unset.
    let v = serde_json::to_value(block(
        "b1",
        BlockKind::Image {
            src: "a.png".into(),
            alt: "".into(),
            width: None,
            height: None,
        },
    ))
    .unwrap();
    assert!(v.get("width").is_none() && v.get("height").is_none());
}

#[test]
fn video() {
    assert_shape(
        BlockKind::Video {
            src: "https://youtu.be/x".into(),
            poster: Some("p.png".into()),
            title: Some("Demo".into()),
            width: Some(640.0),
            height: Some(360.0),
        },
        json!({
            "type": "video", "src": "https://youtu.be/x", "poster": "p.png",
            "title": "Demo", "width": 640.0, "height": 360.0
        }),
    );
}

#[test]
fn math() {
    assert_shape(
        BlockKind::Math {
            tex: "E = mc^2".into(),
        },
        json!({ "type": "math", "tex": "E = mc^2" }),
    );
}

#[test]
fn bar_chart() {
    assert_shape(
        BlockKind::BarChart {
            title: Some("T".into()),
            x_label: Some("X".into()),
            y_label: Some("Y".into()),
            categories: vec!["Q1".into(), "Q2".into()],
            series: vec![ChartSeries {
                name: "North".into(),
                values: vec![42.0, 55.0],
            }],
            y_min: Some(0.0),
            y_max: Some(100.0),
        },
        json!({
            "type": "bar_chart", "title": "T", "x_label": "X", "y_label": "Y",
            "categories": ["Q1", "Q2"],
            "series": [{ "name": "North", "values": [42.0, 55.0] }],
            "y_min": 0.0, "y_max": 100.0
        }),
    );
}

#[test]
fn line_chart() {
    assert_shape(
        BlockKind::LineChart {
            title: None,
            x_label: None,
            y_label: None,
            categories: vec!["Mon".into(), "Tue".into()],
            series: vec![ChartSeries {
                name: "api".into(),
                values: vec![1.0, 2.0],
            }],
            y_min: None,
            y_max: None,
            points: Some(vec![ChartPoint {
                label: "spike".into(),
                category: 1,
                value: 2.0,
            }]),
            point_labels: Some(true),
        },
        json!({
            "type": "line_chart", "categories": ["Mon", "Tue"],
            "series": [{ "name": "api", "values": [1.0, 2.0] }],
            "points": [{ "label": "spike", "category": 1, "value": 2.0 }],
            "point_labels": true
        }),
    );
}

#[test]
fn pie_chart() {
    assert_shape(
        BlockKind::PieChart {
            title: Some("T".into()),
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
        json!({
            "type": "pie_chart", "title": "T",
            "slices": [{ "label": "A", "value": 3.0 }, { "label": "B", "value": 5.0 }]
        }),
    );
}

#[test]
fn diagram() {
    assert_shape(
        BlockKind::Diagram {
            direction: Some(DiagramDirection::Down),
            nodes: vec![
                DiagramNode {
                    id: "a".into(),
                    kind: DiagramNodeKind::Terminator,
                    text: "Start".into(),
                },
                DiagramNode {
                    id: "b".into(),
                    kind: DiagramNodeKind::Decision,
                    text: "OK?".into(),
                },
            ],
            edges: vec![DiagramEdge {
                from: "a".into(),
                to: "b".into(),
                label: Some("go".into()),
                kind: Some(DiagramEdgeKind::Dashed),
            }],
        },
        json!({
            "type": "diagram", "direction": "down",
            "nodes": [
                { "id": "a", "kind": "terminator", "text": "Start" },
                { "id": "b", "kind": "decision", "text": "OK?" }
            ],
            "edges": [{ "from": "a", "to": "b", "label": "go", "kind": "dashed" }]
        }),
    );
}

#[test]
fn sequence_diagram() {
    assert_shape(
        BlockKind::SequenceDiagram {
            participants: vec![
                SeqParticipant {
                    id: "a".into(),
                    name: Some("Client".into()),
                    kind: Some(ParticipantKind::Actor),
                },
                SeqParticipant {
                    id: "b".into(),
                    name: None,
                    kind: None,
                },
            ],
            messages: vec![SeqMessage {
                from: "a".into(),
                to: "b".into(),
                text: Some("hi".into()),
                kind: Some(MessageKind::Async),
            }],
            notes: Some(vec![SeqNote {
                at: 0,
                text: "n".into(),
            }]),
        },
        json!({
            "type": "sequence_diagram",
            "participants": [
                { "id": "a", "name": "Client", "kind": "actor" },
                { "id": "b" }
            ],
            "messages": [{ "from": "a", "to": "b", "text": "hi", "kind": "async" }],
            "notes": [{ "at": 0, "text": "n" }]
        }),
    );
}

#[test]
fn state_diagram() {
    // The wire key is `final` (Rust field `is_final`).
    assert_shape(
        BlockKind::StateDiagram {
            states: vec![
                StateNode {
                    id: "idle".into(),
                    name: Some("Idle".into()),
                    initial: Some(true),
                    is_final: None,
                },
                StateNode {
                    id: "done".into(),
                    name: None,
                    initial: None,
                    is_final: Some(true),
                },
            ],
            transitions: vec![StateTransition {
                from: "idle".into(),
                to: "done".into(),
                trigger: Some("finish".into()),
                guard: Some("ok".into()),
            }],
        },
        json!({
            "type": "state_diagram",
            "states": [
                { "id": "idle", "name": "Idle", "initial": true },
                { "id": "done", "final": true }
            ],
            "transitions": [{ "from": "idle", "to": "done", "trigger": "finish", "guard": "ok" }]
        }),
    );
}

#[test]
fn node_table() {
    assert_shape(
        BlockKind::NodeTable {
            title: "users".into(),
            rows: vec![
                NodeTableRow {
                    key: Some("id".into()),
                    md: "`id` **uuid**".into(),
                },
                NodeTableRow {
                    key: None,
                    md: "plain".into(),
                },
            ],
        },
        json!({
            "type": "node_table", "title": "users",
            "rows": [{ "key": "id", "md": "`id` **uuid**" }, { "md": "plain" }]
        }),
    );
}

#[test]
fn tree() {
    assert_shape(
        BlockKind::Tree {
            nodes: vec![TreeNode {
                title: "src".into(),
                icon: Some("folder".into()),
                children: Some(vec![TreeNode {
                    title: "lib.rs".into(),
                    icon: None,
                    children: None,
                }]),
            }],
        },
        json!({
            "type": "tree",
            "nodes": [{
                "title": "src", "icon": "folder",
                "children": [{ "title": "lib.rs" }]
            }]
        }),
    );
}

#[test]
fn timeline() {
    assert_shape(
        BlockKind::Timeline {
            title: Some("Plan".into()),
            direction: Some(TimelineDirection::Vertical),
            phases: Some(vec![TimelinePhase {
                label: "Alpha".into(),
                from: "2026-01-01".into(),
                to: "2026-03-01".into(),
            }]),
            items: vec![TimelineItem {
                label: "GA".into(),
                on: "2026-06-01".into(),
                side: Some(TimelineSide::Far),
            }],
        },
        json!({
            "type": "timeline", "title": "Plan", "direction": "vertical",
            "phases": [{ "label": "Alpha", "from": "2026-01-01", "to": "2026-03-01" }],
            "items": [{ "label": "GA", "on": "2026-06-01", "side": "far" }]
        }),
    );
    // Optional groups are omitted (not null) when unset.
    let v = serde_json::to_value(block(
        "b1",
        BlockKind::Timeline {
            title: None,
            direction: None,
            phases: None,
            items: vec![],
        },
    ))
    .unwrap();
    assert!(v.get("phases").is_none() && v.get("direction").is_none());
}

#[test]
fn chapter_header() {
    assert_shape(
        BlockKind::ChapterHeader {
            title: "Forge Blocks".into(),
            kicker: Some("Design system".into()),
            reading_time: Some("4 min".into()),
            updated: Some("2026-07-18".into()),
            version: Some("v1".into()),
        },
        json!({
            "type": "chapter_header", "title": "Forge Blocks", "kicker": "Design system",
            "reading_time": "4 min", "updated": "2026-07-18", "version": "v1"
        }),
    );
}

#[test]
fn footnote() {
    // `label`, not `id` — `Block.id` is flattened beside these fields.
    assert_shape(
        BlockKind::Footnote {
            label: "spec".into(),
            md: "See the **fixtures**.".into(),
        },
        json!({ "type": "footnote", "label": "spec", "md": "See the **fixtures**." }),
    );
}

#[test]
fn starter_kinds_roundtrip() {
    // Every starter payload survives the pretty-JSON → parse cycle the JSON
    // source editors use.
    for name in [
        "image",
        "video",
        "math",
        "bar_chart",
        "line_chart",
        "pie_chart",
        "diagram",
        "sequence_diagram",
        "state_diagram",
        "node_table",
        "tree",
        "timeline",
        "chapter_header",
        "footnote",
    ] {
        let kind = forge_blocks::starter_kind(name).expect(name);
        let text = serde_json::to_string_pretty(&kind).unwrap();
        let back: BlockKind = serde_json::from_str(&text).unwrap();
        assert_eq!(back, kind, "starter `{name}` did not round-trip");
        assert_eq!(
            serde_json::to_value(&kind).unwrap()["type"],
            json!(name),
            "starter `{name}` has wrong tag"
        );
    }
    assert!(forge_blocks::starter_kind("nope").is_none());
}

#[test]
fn web_data_fixture_parses() {
    // Data blocks as the web editor writes them, verbatim.
    let text = r##"{
      "version": 1,
      "blocks": [
        { "id": "a", "type": "image", "src": "a.png", "alt": "" },
        { "id": "b", "type": "video", "src": "https://vimeo.com/1" },
        { "id": "c", "type": "math", "tex": "x^2" },
        { "id": "d", "type": "bar_chart", "categories": ["A"], "series": [{ "name": "s", "values": [1] }] },
        { "id": "e", "type": "pie_chart", "slices": [{ "label": "A", "value": 1 }] },
        { "id": "f", "type": "diagram", "nodes": [{ "id": "n", "kind": "process", "text": "N" }], "edges": [] },
        { "id": "g", "type": "state_diagram", "states": [{ "id": "s", "final": true }], "transitions": [] },
        { "id": "h", "type": "tree", "nodes": [{ "title": "root" }] },
        { "id": "i", "type": "timeline", "items": [{ "label": "GA", "on": "2026-06-01" }] },
        { "id": "j", "type": "chapter_header", "title": "T" },
        { "id": "k", "type": "footnote", "label": "n1", "md": "body" }
      ]
    }"##;
    let doc: Document = serde_json::from_str(text).unwrap();
    assert_eq!(doc.blocks.len(), 11);
    assert!(matches!(
        doc.blocks[6].kind,
        BlockKind::StateDiagram { .. }
    ));
    assert!(doc.blocks[0].kind.is_data());
    assert!(doc.blocks[10].kind.is_text());
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
