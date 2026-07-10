//! Overlay stack: modals, sheets, menus, palettes. Overlays paint after the
//! app and receive events first (topmost wins), so they trap focus by
//! construction. Esc closes the top overlay unless it consumed the key.

use crate::event::is_press;
use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Frame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayOutcome {
    /// Not interested — the stack applies default behavior (Esc closes) and
    /// still swallows the event (overlays are modal).
    Ignored,
    /// Handled; stay open.
    Consumed,
    /// Handled; pop this overlay.
    Close,
}

pub trait Overlay {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn handle(&mut self, event: &Event) -> OverlayOutcome;
    /// Dim the content below this overlay (scrim). Defaults to true.
    fn dim_below(&self) -> bool {
        true
    }
}

#[derive(Default)]
pub struct OverlayStack {
    stack: Vec<Box<dyn Overlay>>,
}

impl OverlayStack {
    pub fn new() -> OverlayStack {
        OverlayStack::default()
    }

    pub fn push(&mut self, overlay: Box<dyn Overlay>) {
        self.stack.push(overlay);
    }

    pub fn pop(&mut self) -> Option<Box<dyn Overlay>> {
        self.stack.pop()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Route an event to the topmost overlay. Returns true when the event
    /// was swallowed (i.e. any overlay is open — overlays are modal).
    pub fn handle(&mut self, event: &Event) -> bool {
        let Some(top) = self.stack.last_mut() else {
            return false;
        };
        match top.handle(event) {
            OverlayOutcome::Consumed => {}
            OverlayOutcome::Close => {
                self.stack.pop();
            }
            OverlayOutcome::Ignored => {
                // Default chrome behavior: Esc closes the top overlay.
                if let Event::Key(k) = event {
                    if k.code == KeyCode::Esc && is_press(k) {
                        self.stack.pop();
                    }
                }
            }
        }
        true
    }

    /// Paint the stack bottom-up, dimming beneath each scrim overlay.
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        for overlay in &mut self.stack {
            if overlay.dim_below() {
                dim(frame.buffer_mut(), area, theme);
            }
            overlay.draw(frame, area, theme);
        }
    }
}

/// Scrim: push every cell in `area` down to disabled-text-on-page colors,
/// keeping the glyphs — the terminal equivalent of the web's overlay dim.
pub fn dim(buf: &mut Buffer, area: Rect, theme: &Theme) {
    buf.set_style(area, Style::new().fg(theme.fg[3]).bg(theme.bg[0]));
}
