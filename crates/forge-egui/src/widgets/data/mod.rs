//! Data widgets: tables, logs, trees, boards. Mirrors
//! `forge-tui/src/widgets/data` — same builder APIs and state shapes,
//! adapted to the egui idiom (explicit `FooState`, `.show(ui)`,
//! [`ForgeResponse`](crate::response::ForgeResponse)).

mod accordion;
mod block_grid;
mod file_picker;
mod json_viewer;
mod kanban;
mod key_value;
mod logs;
mod table;
mod tree;

pub use accordion::{Accordion, AccordionState, Collapsible};
pub use block_grid::{BlockGrid, BlockSpec};
pub use file_picker::{FilePicker, FilePickerState};
pub use json_viewer::{JsonViewer, JsonViewerState};
pub use kanban::{Kanban, KanbanCard, KanbanColumn, KanbanMove, KanbanState};
pub use key_value::KeyValue;
pub use logs::{Level, LogLine, Logs, LogsState};
pub use table::{ColWidth, Column, SortDir, Table, TableRow, TableState};
pub use tree::{Tree, TreeNode, TreeState};
