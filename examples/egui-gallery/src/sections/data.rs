//! Data widgets: table, logs, tree, accordion, key/value, JSON viewer.

use forge_egui::prelude::*;
use forge_egui::widgets::{
    Accordion, AccordionState, ColWidth, Collapsible, Column, JsonViewer, JsonViewerState,
    KeyValue, Level, LogLine, Logs, LogsState, SortDir, Table, TableState, Tree, TreeNode,
    TreeState,
};

#[derive(Clone)]
struct Service {
    name: &'static str,
    region: &'static str,
    status: &'static str,
    cpu: f32,
    uptime: &'static str,
}

pub struct DataState {
    table: TableState,
    services: Vec<Service>,
    logs: LogsState,
    lines: Vec<LogLine>,
    tree: TreeState,
    roots: Vec<TreeNode>,
    accordion: AccordionState,
    json: JsonViewerState,
    value: serde_json::Value,
}

impl Default for DataState {
    fn default() -> Self {
        let services = vec![
            Service {
                name: "api-gateway",
                region: "eu-west-1",
                status: "healthy",
                cpu: 42.0,
                uptime: "34d",
            },
            Service {
                name: "auth",
                region: "eu-west-1",
                status: "healthy",
                cpu: 12.5,
                uptime: "34d",
            },
            Service {
                name: "billing",
                region: "us-east-1",
                status: "degraded",
                cpu: 88.1,
                uptime: "2d",
            },
            Service {
                name: "search",
                region: "us-east-1",
                status: "healthy",
                cpu: 61.0,
                uptime: "12d",
            },
            Service {
                name: "ingest",
                region: "ap-south-1",
                status: "healthy",
                cpu: 74.9,
                uptime: "7d",
            },
            Service {
                name: "renderer",
                region: "eu-west-1",
                status: "down",
                cpu: 0.0,
                uptime: "—",
            },
            Service {
                name: "mailer",
                region: "us-west-2",
                status: "healthy",
                cpu: 4.2,
                uptime: "91d",
            },
            Service {
                name: "webhooks",
                region: "us-west-2",
                status: "healthy",
                cpu: 19.7,
                uptime: "91d",
            },
            Service {
                name: "scheduler",
                region: "eu-central-1",
                status: "degraded",
                cpu: 55.3,
                uptime: "1d",
            },
            Service {
                name: "metrics",
                region: "eu-central-1",
                status: "healthy",
                cpu: 33.0,
                uptime: "45d",
            },
            Service {
                name: "docs",
                region: "us-east-1",
                status: "healthy",
                cpu: 2.1,
                uptime: "120d",
            },
            Service {
                name: "cdn-edge",
                region: "ap-south-1",
                status: "healthy",
                cpu: 47.6,
                uptime: "18d",
            },
        ];

        let msgs: [(&str, Level); 8] = [
            ("accepted connection from 10.0.4.18", Level::Debug),
            ("GET /api/v1/deploys 200 12ms", Level::Info),
            ("build #4812 queued on runner-7", Level::Info),
            ("token cache miss for tenant acme", Level::Debug),
            ("retrying webhook delivery (attempt 2)", Level::Warn),
            ("disk pressure on node ingest-3", Level::Warn),
            ("connection reset by peer", Level::Error),
            ("deploy pipeline finished in 94s", Level::Info),
        ];
        let lines = (0..40)
            .map(|i| {
                let (msg, level) = msgs[i % msgs.len()];
                LogLine::new(format!("12:{:02}:{:02}", 30 + i / 60, i % 60), level, msg)
            })
            .collect();

        let roots = vec![TreeNode::new("src", "src")
            .icon(Glyph::Folder)
            .child(
                TreeNode::new("src/widgets", "widgets")
                    .icon(Glyph::Folder)
                    .child(TreeNode::new("src/widgets/table.rs", "table.rs").icon(Glyph::File))
                    .child(TreeNode::new("src/widgets/tree.rs", "tree.rs").icon(Glyph::File))
                    .child(TreeNode::new("src/widgets/logs.rs", "logs.rs").icon(Glyph::File)),
            )
            .child(
                TreeNode::new("src/theme", "theme")
                    .icon(Glyph::Folder)
                    .child(TreeNode::new("src/theme/tokens.rs", "tokens.rs").icon(Glyph::File)),
            )
            .child(TreeNode::new("src/lib.rs", "lib.rs").icon(Glyph::File))
            .child(TreeNode::new("src/main.rs", "main.rs").icon(Glyph::File))];

        let mut tree = TreeState::default();
        tree.expanded.insert("src".into());

        DataState {
            table: TableState::default(),
            services,
            logs: LogsState::default(),
            lines,
            tree,
            roots,
            accordion: AccordionState::default(),
            json: JsonViewerState::default(),
            value: serde_json::json!({
                "service": "api-gateway",
                "replicas": 3,
                "healthy": true,
                "endpoints": [
                    { "path": "/api/v1/deploys", "p99_ms": 41.5 },
                    { "path": "/api/v1/tokens", "p99_ms": 12.0 }
                ],
                "owner": null
            }),
        }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut DataState) {
    let t = Theme::of(ui.ctx());

    Card::new()
        .title("Table — sortable, selectable")
        .show(ui, |ui| {
            // Sorting is the caller's job: order a view of the rows from
            // `state.table.sort`, then hand the sorted view to the widget.
            let mut view: Vec<&Service> = state.services.iter().collect();
            if let Some((col, dir)) = state.table.sort {
                view.sort_by(|a, b| {
                    let ord = match col {
                        0 => a.name.cmp(b.name),
                        1 => a.region.cmp(b.region),
                        2 => a.status.cmp(b.status),
                        3 => a
                            .cpu
                            .partial_cmp(&b.cpu)
                            .unwrap_or(std::cmp::Ordering::Equal),
                        _ => a.uptime.cmp(b.uptime),
                    };
                    if dir == SortDir::Desc {
                        ord.reverse()
                    } else {
                        ord
                    }
                });
            }
            let columns = [
                Column::new("Service").width(ColWidth::Fixed(150.0)),
                Column::new("Region").width(ColWidth::Fixed(130.0)),
                Column::new("Status").width(ColWidth::Fixed(120.0)),
                Column::new("CPU")
                    .width(ColWidth::Fixed(90.0))
                    .align(egui::Align::Max),
                Column::new("Uptime")
                    .width(ColWidth::Remainder)
                    .align(egui::Align::Max),
            ];
            let _ = Table::new(&mut state.table, &columns)
                .striped(true)
                .max_height(280.0)
                .show(ui, view.len(), |row| {
                    let s = view[row.index()];
                    row.text(s.name);
                    row.text(s.region);
                    let tone = match s.status {
                        "healthy" => Tone::Success,
                        "degraded" => Tone::Warning,
                        _ => Tone::Danger,
                    };
                    row.cell(|ui| {
                        let _ = Badge::new(s.status).tone(tone).dot(true).show(ui);
                    });
                    row.text(format!("{:.1}%", s.cpu));
                    row.text(s.uptime);
                });
            if let Some(i) = state.table.selected {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("selected row: {i}"))
                        .font(t.mono(t.type_scale.sm))
                        .color(t.fg[2]),
                );
            }
        });
    ui.add_space(12.0);

    Card::new()
        .title("Logs — follow mode + filter")
        .show(ui, |ui| {
            let _ = Logs::new(&mut state.logs, &state.lines)
                .height(200.0)
                .show(ui);
        });
    ui.add_space(12.0);

    ui.columns(2, |cols| {
        Card::new().title("Tree").show(&mut cols[0], |ui| {
            let _ = Tree::new(&mut state.tree, &state.roots).show(ui);
            if let Some(id) = &state.tree.selected {
                ui.add_space(6.0);
                let t = Theme::of(ui.ctx());
                ui.label(
                    egui::RichText::new(format!("selected: {id}"))
                        .font(t.mono(t.type_scale.sm))
                        .color(t.fg[2]),
                );
            }
        });
        Card::new().title("Key / value").show(&mut cols[1], |ui| {
            let _ = KeyValue::new(&[
                ("Cluster", "prod-eu-1"),
                ("Version", "2024.6.1+f3a91c"),
                ("Replicas", "3 / 3 ready"),
                ("Last deploy", "2h ago by wil"),
            ])
            .show(ui);
            ui.add_space(10.0);
            let _ = KeyValue::new(&[
                ("Commit", "f3a91c7e0b22"),
                ("Digest", "sha256:9f86d081884c"),
            ])
            .mono(true)
            .show(ui);
        });
    });
    ui.add_space(12.0);

    Card::new().title("Accordion & collapsible").show(ui, |ui| {
        let _ = Accordion::new(
            &mut state.accordion,
            &["Build settings", "Environment", "Danger zone"],
        )
        .show(ui, |i, ui| {
            let t = Theme::of(ui.ctx());
            let body = match i {
                0 => "Runner pool, cache keys, and artifact retention.",
                1 => "Secrets and per-branch variable overrides.",
                _ => "Delete this pipeline and all of its history.",
            };
            ui.label(
                egui::RichText::new(body)
                    .font(t.font(
                        ui.ctx(),
                        forge_egui::theme::FontWeight::Regular,
                        t.type_scale.sm,
                    ))
                    .color(t.fg[2]),
            );
        });
        ui.add_space(10.0);
        let _ = Collapsible::new("Advanced options")
            .default_open(false)
            .show(ui, |ui| {
                let t = Theme::of(ui.ctx());
                ui.label(
                    egui::RichText::new("A single disclosure with its own open flag.")
                        .color(t.fg[2]),
                );
            });
    });
    ui.add_space(12.0);

    Card::new().title("JSON viewer").show(ui, |ui| {
        let _ = JsonViewer::new(&mut state.json, &state.value).show(ui);
    });
}
