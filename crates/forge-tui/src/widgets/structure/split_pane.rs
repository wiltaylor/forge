use crate::event::{is_press, Outcome};
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug)]
pub struct SplitState {
    /// First-pane share of the split axis (0.0..=1.0).
    pub ratio: f64,
    last_area: Rect,
    /// Absolute column the divider was painted at, captured by the view on
    /// render — the view's `min` clamp is invisible here, so hit-testing
    /// must use where the divider actually went, not recompute from ratio.
    divider_x: u16,
    dragging: bool,
}

impl Default for SplitState {
    fn default() -> SplitState {
        SplitState {
            ratio: 0.5,
            last_area: Rect::ZERO,
            divider_x: 0,
            dragging: false,
        }
    }
}

impl SplitState {
    pub fn new(ratio: f64) -> SplitState {
        SplitState {
            ratio: ratio.clamp(0.05, 0.95),
            ..Default::default()
        }
    }

    /// The view calls this wherever it lays out: both paint and hit-test
    /// then agree on one divider column.
    fn capture(&mut self, area: Rect, divider_x: u16) {
        self.last_area = area;
        self.divider_x = divider_x;
    }

    fn nudge(&mut self, delta: f64) -> Outcome {
        let next = (self.ratio + delta).clamp(0.05, 0.95);
        if (next - self.ratio).abs() > f64::EPSILON {
            self.ratio = next;
            Outcome::Changed
        } else {
            Outcome::Consumed
        }
    }

    /// ←/→ (with or without Ctrl) resize when the divider is focused.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
            0.15
        } else {
            0.05
        };
        match key.code {
            KeyCode::Left | KeyCode::Up => self.nudge(-step),
            KeyCode::Right | KeyCode::Down => self.nudge(step),
            _ => Outcome::Ignored,
        }
    }

    /// Mouse drag on the divider (requires `RunOptions.mouse`).
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let area = self.last_area;
        if area.is_empty() {
            return Outcome::Ignored;
        }
        match ev.kind {
            MouseEventKind::Down(_)
                if ev.column.abs_diff(self.divider_x) <= 1
                    && (area.y..area.y + area.height).contains(&ev.row) =>
            {
                self.dragging = true;
                Outcome::Consumed
            }
            MouseEventKind::Drag(_) if self.dragging => {
                let rel = ev.column.saturating_sub(area.x) as f64 / area.width.max(1) as f64;
                self.ratio = rel.clamp(0.05, 0.95);
                Outcome::Changed
            }
            MouseEventKind::Up(_) if self.dragging => {
                self.dragging = false;
                Outcome::Consumed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// Two-pane split with a draggable/keyboard-resizable divider. Call
/// [`SplitPane::areas`] for the pane rects, render your content, then render
/// the `SplitPane` itself to paint the divider.
#[derive(Clone, Debug)]
pub struct SplitPane {
    focused: bool,
    min: u16,
}

impl Default for SplitPane {
    fn default() -> SplitPane {
        SplitPane::new()
    }
}

impl SplitPane {
    pub fn new() -> SplitPane {
        SplitPane {
            focused: false,
            min: 8,
        }
    }

    /// Minimum pane width in cells.
    pub fn min(mut self, min: u16) -> Self {
        self.min = min;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    fn divider_x(&self, area: Rect, state: &SplitState) -> u16 {
        let raw = (area.width as f64 * state.ratio) as u16;
        raw.clamp(
            self.min.min(area.width),
            area.width.saturating_sub(self.min + 1),
        )
    }

    /// The two pane rects (left, right) for the current state.
    pub fn areas(&self, area: Rect, state: &mut SplitState) -> (Rect, Rect) {
        let dx = self.divider_x(area, state);
        state.capture(area, area.x + dx);
        let left = Rect::new(area.x, area.y, dx, area.height);
        let right = Rect::new(
            area.x + dx + 1,
            area.y,
            area.width.saturating_sub(dx + 1),
            area.height,
        );
        (left, right)
    }
}

impl StatefulWidget for SplitPane {
    type State = SplitState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut SplitState) {
        paint(area, |t| {
            let divider_x = area.x + self.divider_x(area, state);
            state.capture(area, divider_x);
            let color = if self.focused {
                t.accent.base
            } else {
                t.border.default
            };
            for dy in 0..area.height {
                buf.set_string(divider_x, area.y + dy, "│", Style::new().fg(color));
            }
        });
    }
}
