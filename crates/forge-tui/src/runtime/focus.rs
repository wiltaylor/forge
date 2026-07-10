//! Immediate-mode focus management. Widgets re-register every frame in
//! render order (the "DOM order"); Tab traversal walks that order. The
//! runtime calls [`FocusRing::begin_frame`] before each `App::draw`.

/// Identifies a focusable widget: a static name plus an index for repeated
/// widgets (list rows, dynamic fields).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FocusId {
    name: &'static str,
    index: u32,
}

impl FocusId {
    pub const fn new(name: &'static str) -> FocusId {
        FocusId { name, index: 0 }
    }

    pub const fn indexed(name: &'static str, index: u32) -> FocusId {
        FocusId { name, index }
    }
}

#[derive(Debug, Default)]
pub struct FocusRing {
    /// Registration order accumulated during the current frame.
    order: Vec<FocusId>,
    /// The last complete frame's order — what traversal walks.
    prev: Vec<FocusId>,
    current: Option<FocusId>,
}

impl FocusRing {
    pub fn new() -> FocusRing {
        FocusRing::default()
    }

    /// Start a new frame; the finished frame becomes the traversal order.
    pub fn begin_frame(&mut self) {
        if !self.order.is_empty() {
            self.prev = std::mem::take(&mut self.order);
        } else {
            self.order.clear();
        }
    }

    /// Declare a focusable widget (call during render, in visual order).
    /// Returns whether it currently has focus — pass that to the widget's
    /// `.focused(...)` builder. The first widget ever registered gets focus.
    pub fn register(&mut self, id: FocusId) -> bool {
        self.order.push(id);
        if self.current.is_none() {
            self.current = Some(id);
        }
        self.current == Some(id)
    }

    pub fn current(&self) -> Option<FocusId> {
        self.current
    }

    pub fn is(&self, id: FocusId) -> bool {
        self.current == Some(id)
    }

    pub fn focus(&mut self, id: FocusId) {
        self.current = Some(id);
    }

    fn ring(&self) -> &[FocusId] {
        if self.prev.is_empty() {
            &self.order
        } else {
            &self.prev
        }
    }

    fn step(&mut self, delta: isize) {
        let ring = self.ring();
        if ring.is_empty() {
            return;
        }
        let len = ring.len() as isize;
        let pos = self
            .current
            .and_then(|c| ring.iter().position(|&id| id == c))
            .map(|p| p as isize)
            .unwrap_or(-delta); // unknown current → land on first/last
        let next = (pos + delta).rem_euclid(len) as usize;
        self.current = Some(self.ring()[next]);
    }

    /// Focus the next widget in render order (wraps).
    pub fn next(&mut self) {
        self.step(1);
    }

    /// Focus the previous widget in render order (wraps).
    pub fn prev(&mut self) {
        self.step(-1);
    }
}
