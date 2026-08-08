//! The two lines every widget's `render` used to open with.
//!
//! A ratatui `render` is handed an area and a buffer and nothing else, so each
//! widget began by asking the same two questions: is there anywhere to paint,
//! and what colours do I paint with. One copy of the answer per widget is one
//! place to edit per widget when the answer changes. [`paint`] holds it once.
//!
//! A widget that measures rather than paints — `Card::inner`, `Markdown::height`
//! — has no area to guard and reads [`ambient_theme`] directly.

use crate::theme::{ambient_theme, Theme};
use ratatui::layout::Rect;

/// Run `block` with the [ambient theme](ambient_theme), unless `area` has no
/// cells to paint.
///
/// A zero-width or zero-height area is routine, not an error: a `Layout` hands
/// one to any widget it squeezed out, and a hidden pane is a pane with no
/// cells. Every widget answered that with an early return, so the early return
/// lives here instead.
///
/// The theme is snapshotted once per call, so a
/// [`set_ambient_theme`](crate::theme::set_ambient_theme) from another thread
/// cannot change the colours a widget is halfway through painting with. It can
/// still land between two widgets of one frame, which is why an app swaps
/// between frames rather than during one.
///
/// ```
/// use forge_tui::widgets::paint;
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use ratatui::style::Style;
///
/// let area = Rect::new(0, 0, 4, 1);
/// let mut buf = Buffer::empty(area);
/// paint(area, |t| {
///     buf.set_string(0, 0, "ok", Style::new().fg(t.accent.base));
/// });
/// ```
pub fn paint(area: Rect, block: impl FnOnce(&Theme)) {
    if area.is_empty() {
        return;
    }
    block(&ambient_theme());
}
