//! Render-time geometry and the shared hit-tests against it.
//!
//! A widget that reacts to the mouse remembers, in its state, the rectangles
//! it painted its clickable parts at, and tests later mouse events against
//! them. That only works in render-then-dispatch order: the rectangles are
//! captured during `render`, so a mouse event dispatched before the first
//! render — or after a resize but before the next render — is tested against
//! stale geometry. This module holds that contract in one place instead of
//! leaving it implicit in every widget: an unrendered [`RectCache`] is empty
//! and every hit misses, each `render` rebuilds the cache from scratch, and
//! so the geometry is honest again one frame after any relayout.

use crate::event::{clicked, is_press, Outcome};
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;

/// The rectangles a widget painted its clickable items at, in item order.
///
/// `render` rebuilds the cache every frame — [`clear`](RectCache::clear)
/// first, then one [`push`](RectCache::push) per item painted — and the
/// mouse handler resolves an event with [`hit`](RectCache::hit) or
/// [`select`](RectCache::select). Before the first render the cache is
/// empty, so every event misses; that is the correct answer to a click on
/// geometry that has never been painted.
///
/// ```
/// use forge_tui::event::Outcome;
/// use forge_tui::widgets::RectCache;
/// use ratatui::crossterm::event::{
///     KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
/// };
/// use ratatui::layout::Rect;
///
/// let click = MouseEvent {
///     kind: MouseEventKind::Down(MouseButton::Left),
///     column: 5,
///     row: 0,
///     modifiers: KeyModifiers::NONE,
/// };
/// let mut cache = RectCache::default();
/// let mut selected = 0;
/// // Not rendered yet: nothing to hit.
/// assert_eq!(cache.select(&click, &mut selected), Outcome::Ignored);
/// // What a render records...
/// cache.push(Rect::new(0, 0, 3, 1));
/// cache.push(Rect::new(4, 0, 3, 1));
/// // ...a later click resolves against.
/// assert_eq!(cache.select(&click, &mut selected), Outcome::Changed);
/// assert_eq!(selected, 1);
/// ```
#[derive(Clone, Debug, Default)]
pub struct RectCache {
    rects: Vec<Rect>,
}

impl RectCache {
    /// Drop the previous frame's geometry. Call at the top of `render`,
    /// before the first [`push`](RectCache::push).
    pub fn clear(&mut self) {
        self.rects.clear();
    }

    /// Record the rectangle the next item was painted at.
    pub fn push(&mut self, rect: Rect) {
        self.rects.push(rect);
    }

    /// The index of the cached rectangle a left click landed in.
    pub fn hit(&self, ev: &MouseEvent) -> Option<usize> {
        self.rects.iter().position(|rect| clicked(ev, *rect))
    }

    /// The shared click-to-select mouse handler: a left click inside a
    /// cached rectangle selects that index — `Changed` if the selection
    /// moved, `Consumed` if it was already selected. Everything else is
    /// `Ignored`, including every event that arrives before the first
    /// render, when the cache is still empty.
    pub fn select(&self, ev: &MouseEvent, selected: &mut usize) -> Outcome {
        match self.hit(ev) {
            Some(i) if *selected != i => {
                *selected = i;
                Outcome::Changed
            }
            Some(_) => Outcome::Consumed,
            None => Outcome::Ignored,
        }
    }
}

/// State shared by every click-to-toggle control: one boolean plus the
/// rectangle the control was painted at — a single-rect [`RectCache`], with
/// the same contract. Until `render` has called
/// [`set_area`](ToggleState::set_area) the control has no geometry, so
/// clicks miss.
///
/// [`Toggle`](crate::widgets::Toggle) and
/// [`Checkbox`](crate::widgets::Checkbox) use this state directly;
/// [`CollapsibleState`](crate::widgets::CollapsibleState) builds on it and
/// adds the disclosure arrow keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct ToggleState {
    pub on: bool,
    area: Rect,
}

impl ToggleState {
    pub fn new(on: bool) -> ToggleState {
        ToggleState {
            on,
            area: Rect::default(),
        }
    }

    /// Record the clickable rectangle. Call from `render`.
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    /// Click anywhere on the control toggles it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if clicked(ev, self.area) {
            self.on = !self.on;
            Outcome::Changed
        } else {
            Outcome::Ignored
        }
    }

    /// Space/Enter toggles.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.on = !self.on;
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}
