//! Specialty widgets: markdown, code viewing, the chat kit, the block page
//! editor, flowcharts, and the node-graph editor — the egui siblings of
//! `@forge/markdown`, `@forge/code`, `@forge/chat`, `@forge/blocks`, the web
//! `Flowchart`, and `@forge/graph`'s `NodeGraph`. Markdown/chat/code/blocks
//! sit behind cargo features (matching forge-tui's feature shape); the
//! flowchart and node graph are dependency-free and always available.

#[cfg(feature = "blocks")]
mod blocks;
#[cfg(feature = "chat")]
mod chat;
#[cfg(feature = "code")]
mod code;
mod flowchart;
#[cfg(feature = "markdown")]
mod markdown;
mod node_graph;

#[cfg(feature = "blocks")]
pub use blocks::*;
#[cfg(feature = "chat")]
pub use chat::*;
#[cfg(feature = "code")]
pub use code::*;
pub use flowchart::*;
#[cfg(feature = "markdown")]
pub use markdown::*;
pub use node_graph::*;
