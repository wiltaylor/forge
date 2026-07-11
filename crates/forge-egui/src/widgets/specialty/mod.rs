//! Specialty widgets: markdown, code viewing, the chat kit, flowcharts, and
//! the node-graph editor — the egui siblings of `@forge/markdown`,
//! `@forge/code`, `@forge/chat`, the web `Flowchart`, and `@forge/graph`'s
//! `NodeGraph`. Markdown/chat/code sit behind cargo features (matching
//! forge-tui's feature shape); the flowchart and node graph are
//! dependency-free and always available.

#[cfg(feature = "chat")]
mod chat;
#[cfg(feature = "code")]
mod code;
mod flowchart;
#[cfg(feature = "markdown")]
mod markdown;
mod node_graph;

#[cfg(feature = "chat")]
pub use chat::*;
#[cfg(feature = "code")]
pub use code::*;
pub use flowchart::*;
#[cfg(feature = "markdown")]
pub use markdown::*;
pub use node_graph::*;
