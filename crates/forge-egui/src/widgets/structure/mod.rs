//! Page structure: breadcrumbs, page headers, tabs, pagination, settings
//! rows, split panes, and the standalone status bar.

mod crumbs;
mod page_head;
mod pagination;
mod settings;
mod split_pane;
mod status_bar;
mod tabs;

pub use crumbs::Crumbs;
pub use page_head::PageHead;
pub use pagination::Pagination;
pub use settings::{SettingsRow, SettingsSection};
pub use split_pane::{SplitPane, SplitState};
pub use status_bar::StatusBar;
pub use tabs::{TabItem, Tabs};
