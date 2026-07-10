use forge_tui::prelude::*;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use serde_json::json;

const TABLE: FocusId = FocusId::new("dt-table");
const LOGS: FocusId = FocusId::new("dt-logs");
const TREE: FocusId = FocusId::new("dt-tree");
const JSON: FocusId = FocusId::new("dt-json");
const ACC: FocusId = FocusId::new("dt-acc");

pub struct DataState {
    pub table: TableState,
    pub logs: LogsState,
    pub tree: TreeState,
    pub json: JsonViewerState,
    pub acc: AccordionState,
    pub rows: Vec<Vec<String>>,
    pub log_lines: Vec<LogLine>,
    pub json_value: serde_json::Value,
}

impl Default for DataState {
    fn default() -> DataState {
        let rows = vec![
            vec!["node-1".into(), "ready".into(), "13".into(), "42%".into()],
            vec!["node-2".into(), "ready".into(), "21".into(), "67%".into()],
            vec!["node-3".into(), "draining".into(), "4".into(), "12%".into()],
            vec!["node-4".into(), "down".into(), "0".into(), "0%".into()],
            vec!["node-5".into(), "ready".into(), "18".into(), "55%".into()],
            vec!["node-6".into(), "ready".into(), "9".into(), "31%".into()],
        ];
        let log_lines = vec![
            LogLine::new(Level::Info, "server listening on :8765").ts("10:41:02"),
            LogLine::new(Level::Debug, "doc store loaded 14 documents").ts("10:41:02"),
            LogLine::new(Level::Info, "event bus online").ts("10:41:03"),
            LogLine::new(Level::Warn, "event bus lagging: 2 subscribers behind").ts("10:41:19"),
            LogLine::new(Level::Error, "health check failed on node-3").ts("10:41:21"),
            LogLine::new(Level::Info, "node-3 marked draining").ts("10:41:21"),
            LogLine::new(Level::Trace, "gc pass finished in 3ms").ts("10:41:30"),
            LogLine::new(Level::Info, "deploy gallery@0.1 queued").ts("10:41:44"),
        ];
        let json_value = json!({
            "cluster": "forge-prod",
            "nodes": [
                {"name": "node-1", "ready": true, "pods": 13},
                {"name": "node-3", "ready": false, "pods": 4}
            ],
            "version": 3,
            "flags": {"auto_heal": true, "canary": null}
        });
        let mut state = DataState {
            table: TableState::new(),
            logs: LogsState::new(),
            tree: TreeState::new(),
            json: JsonViewerState::new(),
            acc: AccordionState::new(),
            rows,
            log_lines,
            json_value,
        };
        state.tree.expand(&[0]);
        state
    }
}

pub const TREE_ROOTS: fn() -> [TreeNode<'static>; 2] = || {
    const SERVICES: [TreeNode<'static>; 3] = [
        TreeNode { label: "forge-server", children: &[] },
        TreeNode { label: "forge-auth", children: &[] },
        TreeNode { label: "gallery", children: &[] },
    ];
    const WORKERS: [TreeNode<'static>; 2] = [
        TreeNode { label: "builder-1", children: &[] },
        TreeNode { label: "builder-2", children: &[] },
    ];
    [
        TreeNode { label: "services", children: &SERVICES },
        TreeNode { label: "workers", children: &WORKERS },
    ]
};

impl DataState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        let outcome = match focused {
            Some(id) if id == TABLE => {
                let out = self.table.handle_key(key);
                if out == Outcome::Changed {
                    self.apply_sort();
                }
                out
            }
            Some(id) if id == LOGS => self.logs.handle_key(key),
            Some(id) if id == TREE => self.tree.handle_key(key, &TREE_ROOTS()),
            Some(id) if id == JSON => self.json.handle_key(key, &self.json_value.clone()),
            Some(id) if id == ACC => self.acc.handle_key(key),
            _ => Outcome::Ignored,
        };
        if outcome == Outcome::Submitted {
            if focused == Some(TABLE) {
                let row = self.rows.get(self.table.cursor);
                if let Some(row) = row {
                    ctx.toast().info(format!("Open {}", row[0]));
                }
            }
            return Outcome::Consumed;
        }
        outcome
    }

    fn apply_sort(&mut self) {
        if let Some((col, asc)) = self.table.sort {
            self.rows.sort_by(|a, b| {
                let (x, y) = (a.get(col), b.get(col));
                // Numeric-aware compare for count/cpu columns.
                let ord = match (
                    x.and_then(|v| v.trim_end_matches('%').parse::<f64>().ok()),
                    y.and_then(|v| v.trim_end_matches('%').parse::<f64>().ok()),
                ) {
                    (Some(nx), Some(ny)) => nx.partial_cmp(&ny).unwrap_or(std::cmp::Ordering::Equal),
                    _ => x.cmp(&y),
                };
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut DataState) {
    let f_table = ctx.focus.register(TABLE);
    let f_logs = ctx.focus.register(LOGS);
    let f_tree = ctx.focus.register(TREE);
    let f_json = ctx.focus.register(JSON);
    let f_acc = ctx.focus.register(ACC);

    let half_h = area.height / 2;
    let top = Rect::new(area.x, area.y, area.width, half_h.saturating_sub(1));
    let bottom = Rect::new(area.x, area.y + half_h, area.width, area.height - half_h);

    // Top: table (left) + logs (right).
    let cols = Grid::new(2).gap(2).cells(top, 2, top.height);
    if top.height > 2 {
        frame.render_widget(Eyebrow::new("Table — s sort · Space select").theme(t), Rect::new(cols[0].x, cols[0].y, cols[0].width, 1));
        let columns = [
            Column::new("node"),
            Column::new("state"),
            Column::new("pods").width(5).right(),
            Column::new("cpu").width(5).right(),
        ];
        let rows: Vec<Vec<&str>> = state
            .rows
            .iter()
            .map(|r| r.iter().map(String::as_str).collect())
            .collect();
        frame.render_stateful_widget(
            Table::new(&columns, &rows).focused(f_table).theme(t),
            Rect::new(cols[0].x, cols[0].y + 1, cols[0].width, cols[0].height - 1),
            &mut state.table,
        );

        frame.render_widget(Eyebrow::new("Logs — f follow · ↑ scroll").theme(t), Rect::new(cols[1].x, cols[1].y, cols[1].width, 1));
        state.logs.search = Some("node-3".into());
        frame.render_stateful_widget(
            Logs::new(&state.log_lines).focused(f_logs).theme(t),
            Rect::new(cols[1].x, cols[1].y + 1, cols[1].width, cols[1].height - 1),
            &mut state.logs,
        );
    }

    // Bottom: tree | json | keyvalue+accordion.
    let cols = Grid::new(3).gap(2).cells(bottom, 3, bottom.height);
    if bottom.height > 2 {
        frame.render_widget(Eyebrow::new("Tree").theme(t), Rect::new(cols[0].x, cols[0].y, cols[0].width, 1));
        frame.render_stateful_widget(
            Tree::new(&TREE_ROOTS()).focused(f_tree).theme(t),
            Rect::new(cols[0].x, cols[0].y + 1, cols[0].width, cols[0].height - 1),
            &mut state.tree,
        );

        frame.render_widget(Eyebrow::new("JsonViewer").theme(t), Rect::new(cols[1].x, cols[1].y, cols[1].width, 1));
        let value = state.json_value.clone();
        JsonViewer::new().focused(f_json).theme(t).render_value(
            &value,
            Rect::new(cols[1].x, cols[1].y + 1, cols[1].width, cols[1].height - 1),
            frame.buffer_mut(),
            &mut state.json,
        );

        frame.render_widget(Eyebrow::new("KeyValue · Accordion").theme(t), Rect::new(cols[2].x, cols[2].y, cols[2].width, 1));
        frame.render_widget(
            KeyValue::new(&[("cluster", "forge-prod"), ("region", "eu-west-1"), ("uptime", "42d")]).theme(t),
            Rect::new(cols[2].x, cols[2].y + 1, cols[2].width, 3),
        );
        frame.render_stateful_widget(
            Accordion::new(&[
                ("Rollout policy", "Canary 10% for 15 minutes, then full."),
                ("Alerting", "Page on-call when error rate exceeds 1%."),
            ])
            .focused(f_acc)
            .theme(t),
            Rect::new(cols[2].x, cols[2].y + 5, cols[2].width, cols[2].height.saturating_sub(5)),
            &mut state.acc,
        );
    }
}
