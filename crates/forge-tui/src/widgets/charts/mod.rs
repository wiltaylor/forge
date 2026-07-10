//! Zero-dep themed charts on the locked CVD-safe series palette
//! `[accent, danger, success, warning, info]` (overflow folds into `fg[2]`
//! "Other" — never reorder, never cycle). Bar/Line/Sparkline wrap the
//! ratatui primitives; Pie and Gantt are drawn directly.

mod bar;
mod gantt;
mod legend;
mod line;
mod pie;
mod sparkline;

pub use bar::BarChart;
pub use gantt::{Gantt, GanttTask};
pub use legend::Legend;
pub use line::{LineChart, LineSeries};
pub use pie::{PieChart, PieSlice};
pub use sparkline::Sparkline;
