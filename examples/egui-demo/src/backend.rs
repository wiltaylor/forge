//! The in-process forge-core backend: doc store, actions, event bus, and the
//! async↔frame-loop bridges (`Job`, the event forwarder). No HTTP anywhere —
//! this is what a native Rust Forge app looks like.

use forge_core::{box_action, ActionCtx, BoxedAction, Claims, DocStore, EventBus, ForgeError};
use forge_egui::egui;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct Backend {
    /// Owned runtime — dropping it on window close aborts every session/task.
    pub rt: Runtime,
    pub store: DocStore,
    pub bus: EventBus,
    pub actions: BTreeMap<&'static str, BoxedAction>,
}

impl Backend {
    pub fn new() -> Backend {
        let rt = Runtime::new().expect("tokio runtime");
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        std::fs::create_dir_all(&data_dir).expect("create data dir");
        let store = DocStore::new(data_dir);
        let bus = EventBus::new();

        let mut actions: BTreeMap<&'static str, BoxedAction> = BTreeMap::new();
        actions.insert(
            "echo",
            box_action(|payload, _ctx| async move { Ok(json!({ "echo": payload })) }),
        );
        actions.insert(
            "system_info",
            box_action(|_payload, _ctx| async move {
                Ok(json!({
                    "os": std::env::consts::OS,
                    "arch": std::env::consts::ARCH,
                    "cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
                }))
            }),
        );
        actions.insert(
            "publish",
            box_action(|payload, ctx| async move {
                let topic = payload
                    .get("topic")
                    .and_then(Value::as_str)
                    .unwrap_or("demo")
                    .to_owned();
                let data = payload.get("data").cloned().unwrap_or(Value::Null);
                ctx.events.publish(&topic, data);
                Ok(json!({ "published": topic }))
            }),
        );

        // Live-telemetry ticker, same shape as rust-demo's.
        let tick_bus = bus.clone();
        rt.spawn(async move {
            let mut n = 0u64;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                n += 1;
                tick_bus.publish("ticks", json!({ "n": n, "source": "egui-demo" }));
            }
        });

        Backend {
            rt,
            store,
            bus,
            actions,
        }
    }

    /// Invoke a registered action exactly as a Forge server would.
    pub fn invoke(&self, egui: &egui::Context, name: &str, payload: Value) -> Job<Value> {
        let action = self.actions.get(name).cloned();
        let ctx = ActionCtx {
            claims: Claims::anonymous(),
            events: self.bus.clone(),
        };
        let name = name.to_owned();
        spawn_job(self, egui, async move {
            match action {
                Some(action) => (action)(payload, ctx).await,
                None => Err(forge_core::unknown_action_error(&name, &[])),
            }
        })
    }
}

/// A background operation the UI polls once per frame — the async↔immediate
/// bridge. Never `block_on` on the UI thread; spawn, then `poll()`.
pub struct Job<T> {
    rx: mpsc::Receiver<Result<T, ForgeError>>,
}

impl<T> Job<T> {
    pub fn poll(&self) -> Option<Result<T, ForgeError>> {
        self.rx.try_recv().ok()
    }
}

pub fn spawn_job<T: Send + 'static>(
    backend: &Backend,
    egui: &egui::Context,
    fut: impl std::future::Future<Output = Result<T, ForgeError>> + Send + 'static,
) -> Job<T> {
    let (tx, rx) = mpsc::channel();
    let egui = egui.clone();
    backend.rt.spawn(async move {
        let result = fut.await;
        let _ = tx.send(result);
        egui.request_repaint();
    });
    Job { rx }
}

/// One event as the UI sees it.
pub struct FeedEntry {
    pub topic: String,
    pub json: String,
}

/// Bridges the tokio broadcast bus into the frame loop: a forwarder task
/// pushes into a std channel and wakes the UI; `drain()` is called per frame.
pub struct EventFeed {
    rx: mpsc::Receiver<FeedEntry>,
    pub entries: Vec<FeedEntry>,
    pub lagged: u64,
}

const FEED_CAP: usize = 200;

impl EventFeed {
    pub fn start(backend: &Backend, egui: &egui::Context) -> EventFeed {
        let (tx, rx) = mpsc::channel();
        let mut sub = backend.bus.subscribe();
        let egui = egui.clone();
        backend.rt.spawn(async move {
            loop {
                match sub.recv().await {
                    Ok(event) => {
                        let event: Arc<forge_core::Event> = event;
                        let entry = FeedEntry {
                            topic: event.topic.clone(),
                            json: event.json.clone(),
                        };
                        if tx.send(entry).is_err() {
                            break; // UI gone
                        }
                        egui.request_repaint();
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let entry = FeedEntry {
                            topic: "(lagged)".to_owned(),
                            json: format!("{{\"dropped\":{n}}}"),
                        };
                        if tx.send(entry).is_err() {
                            break;
                        }
                        egui.request_repaint();
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        EventFeed {
            rx,
            entries: Vec::new(),
            lagged: 0,
        }
    }

    /// Pull pending events into the ring buffer (newest last, capped).
    pub fn drain(&mut self) {
        while let Ok(entry) = self.rx.try_recv() {
            if entry.topic == "(lagged)" {
                self.lagged += 1;
            }
            self.entries.push(entry);
        }
        if self.entries.len() > FEED_CAP {
            let excess = self.entries.len() - FEED_CAP;
            self.entries.drain(..excess);
        }
    }
}
