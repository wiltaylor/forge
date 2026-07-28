//! A rich sample document exercising every block kind — shared by the TUI
//! and egui gallery sections (and mirrored by the web demo) so all platforms
//! demo the same content.

use crate::schema::{
    Block, BlockKind, ChartSeries, Column, DiagramEdge, DiagramNode, DiagramNodeKind, Document,
    ListStyle, MessageKind, NodeTableRow, PieSlice, SeqMessage, SeqParticipant, StateNode,
    StateTransition, TimelineItem, TimelinePhase, Tone, TreeNode,
};

fn b(kind: BlockKind) -> Block {
    Block::new(kind)
}

/// Every block kind, inline styles, emoji, and a two-column layout.
pub fn sample_document() -> Document {
    Document::from_blocks(vec![
        b(BlockKind::ChapterHeader {
            title: "Forge Blocks".into(),
            kicker: Some("Design system".into()),
            reading_time: Some("4 min".into()),
            updated: Some("2026-07-18".into()),
            version: Some("v1".into()),
        }),
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
        b(BlockKind::Heading {
            level: 2,
            md: "Media".into(),
        }),
        b(BlockKind::Image {
            src: "https://picsum.photos/seed/forge/640/360".into(),
            alt: "A random landscape".into(),
            width: Some(640.0),
            height: Some(360.0),
        }),
        b(BlockKind::Video {
            src: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
            poster: None,
            title: Some("Launch demo".into()),
            width: None,
            height: None,
        }),
        b(BlockKind::Math {
            tex: "\\frac{1}{N}\\sum_{i=1}^{N} (y_i - \\hat{y}_i)^2".into(),
        }),
        b(BlockKind::Heading {
            level: 2,
            md: "Charts".into(),
        }),
        b(BlockKind::BarChart {
            title: Some("Quarterly revenue ($k)".into()),
            x_label: None,
            y_label: None,
            categories: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
            series: vec![
                ChartSeries {
                    name: "North".into(),
                    values: vec![42.0, 55.0, 61.0, 78.0],
                },
                ChartSeries {
                    name: "South".into(),
                    values: vec![30.0, 48.0, 52.0, 66.0],
                },
            ],
            y_min: Some(0.0),
            y_max: Some(100.0),
        }),
        b(BlockKind::LineChart {
            title: Some("Latency p95 (ms)".into()),
            x_label: None,
            y_label: None,
            categories: vec![
                "Mon".into(),
                "Tue".into(),
                "Wed".into(),
                "Thu".into(),
                "Fri".into(),
            ],
            series: vec![
                ChartSeries {
                    name: "api".into(),
                    values: vec![120.0, 132.0, 101.0, 134.0, 90.0],
                },
                ChartSeries {
                    name: "web".into(),
                    values: vec![220.0, 182.0, 191.0, 234.0, 150.0],
                },
            ],
            y_min: None,
            y_max: None,
            points: None,
            point_labels: None,
        }),
        b(BlockKind::PieChart {
            title: Some("Traffic by source".into()),
            slices: vec![
                PieSlice {
                    label: "Search".into(),
                    value: 46.0,
                },
                PieSlice {
                    label: "Direct".into(),
                    value: 32.0,
                },
                PieSlice {
                    label: "Referral".into(),
                    value: 14.0,
                },
                PieSlice {
                    label: "Social".into(),
                    value: 8.0,
                },
            ],
        }),
        b(BlockKind::Heading {
            level: 2,
            md: "Diagrams".into(),
        }),
        b(BlockKind::Diagram {
            direction: None,
            nodes: vec![
                DiagramNode {
                    id: "start".into(),
                    kind: DiagramNodeKind::Terminator,
                    text: "Start".into(),
                },
                DiagramNode {
                    id: "build".into(),
                    kind: DiagramNodeKind::Process,
                    text: "Build".into(),
                },
                DiagramNode {
                    id: "ok".into(),
                    kind: DiagramNodeKind::Decision,
                    text: "Tests pass?".into(),
                },
                DiagramNode {
                    id: "ship".into(),
                    kind: DiagramNodeKind::Terminator,
                    text: "Ship".into(),
                },
            ],
            edges: vec![
                DiagramEdge {
                    from: "start".into(),
                    to: "build".into(),
                    label: None,
                    kind: None,
                },
                DiagramEdge {
                    from: "build".into(),
                    to: "ok".into(),
                    label: None,
                    kind: None,
                },
                DiagramEdge {
                    from: "ok".into(),
                    to: "ship".into(),
                    label: Some("yes".into()),
                    kind: None,
                },
                DiagramEdge {
                    from: "ok".into(),
                    to: "build".into(),
                    label: Some("no".into()),
                    kind: None,
                },
            ],
        }),
        b(BlockKind::SequenceDiagram {
            participants: vec![
                SeqParticipant {
                    id: "cli".into(),
                    name: Some("Client".into()),
                    kind: None,
                },
                SeqParticipant {
                    id: "api".into(),
                    name: Some("API".into()),
                    kind: None,
                },
                SeqParticipant {
                    id: "db".into(),
                    name: Some("DB".into()),
                    kind: None,
                },
            ],
            messages: vec![
                SeqMessage {
                    from: "cli".into(),
                    to: "api".into(),
                    text: Some("POST /login".into()),
                    kind: None,
                },
                SeqMessage {
                    from: "api".into(),
                    to: "db".into(),
                    text: Some("SELECT user".into()),
                    kind: Some(MessageKind::Sync),
                },
                SeqMessage {
                    from: "db".into(),
                    to: "api".into(),
                    text: Some("row".into()),
                    kind: Some(MessageKind::Reply),
                },
                SeqMessage {
                    from: "api".into(),
                    to: "cli".into(),
                    text: Some("200 + token".into()),
                    kind: Some(MessageKind::Reply),
                },
            ],
            notes: None,
        }),
        b(BlockKind::StateDiagram {
            states: vec![
                StateNode {
                    id: "idle".into(),
                    name: Some("Idle".into()),
                    initial: Some(true),
                    is_final: None,
                },
                StateNode {
                    id: "running".into(),
                    name: Some("Running".into()),
                    initial: None,
                    is_final: None,
                },
                StateNode {
                    id: "done".into(),
                    name: Some("Done".into()),
                    initial: None,
                    is_final: Some(true),
                },
            ],
            transitions: vec![
                StateTransition {
                    from: "idle".into(),
                    to: "running".into(),
                    trigger: Some("start".into()),
                    guard: None,
                },
                StateTransition {
                    from: "running".into(),
                    to: "done".into(),
                    trigger: Some("finish".into()),
                    guard: Some("ok".into()),
                },
                StateTransition {
                    from: "running".into(),
                    to: "idle".into(),
                    trigger: Some("abort".into()),
                    guard: None,
                },
            ],
        }),
        b(BlockKind::NodeTable {
            title: "users".into(),
            rows: vec![
                NodeTableRow {
                    key: Some("id".into()),
                    md: "`id` **uuid** pk".into(),
                },
                NodeTableRow {
                    key: Some("email".into()),
                    md: "`email` **text** unique".into(),
                },
                NodeTableRow {
                    key: None,
                    md: "`created_at` **timestamptz**".into(),
                },
            ],
        }),
        b(BlockKind::Tree {
            nodes: vec![TreeNode {
                title: "src".into(),
                icon: None,
                children: Some(vec![
                    TreeNode {
                        title: "lib.rs".into(),
                        icon: None,
                        children: None,
                    },
                    TreeNode {
                        title: "widgets".into(),
                        icon: None,
                        children: Some(vec![TreeNode {
                            title: "blocks".into(),
                            icon: None,
                            children: None,
                        }]),
                    },
                ]),
            }],
        }),
        b(BlockKind::Heading {
            level: 2,
            md: "Timeline".into(),
        }),
        b(BlockKind::Timeline {
            title: Some("Release plan".into()),
            direction: None,
            phases: Some(vec![
                TimelinePhase {
                    label: "Alpha".into(),
                    from: "2026-01-01".into(),
                    to: "2026-03-01".into(),
                },
                TimelinePhase {
                    label: "Beta".into(),
                    from: "2026-03-01".into(),
                    to: "2026-06-01".into(),
                },
            ]),
            items: vec![
                TimelineItem {
                    label: "Kickoff".into(),
                    on: "2026-01-01".into(),
                    side: None,
                },
                TimelineItem {
                    label: "Feature freeze".into(),
                    on: "2026-04-15".into(),
                    side: None,
                },
                TimelineItem {
                    label: "GA".into(),
                    on: "2026-06-01".into(),
                    side: None,
                },
            ],
        }),
        b(BlockKind::Paragraph {
            md: "Footnotes get inline references[^spec] that link to definitions.".into(),
        }),
        b(BlockKind::Footnote {
            label: "spec".into(),
            md: "See the frozen JSON fixtures in `tests/schema.rs`.".into(),
        }),
    ])
}
