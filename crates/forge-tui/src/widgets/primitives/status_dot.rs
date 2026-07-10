use crate::text;
use crate::theme::{default_theme, Severity, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Colored presence dot with optional label; `pulse` blinks it on the
/// animation tick (pass the runtime frame counter).
#[derive(Clone, Debug)]
pub struct StatusDot<'a> {
    severity: Severity,
    color: Option<Color>,
    label: Option<&'a str>,
    pulse: bool,
    frame: u64,
    theme: Option<&'a Theme>,
}

impl<'a> StatusDot<'a> {
    pub fn new(severity: Severity) -> StatusDot<'a> {
        StatusDot {
            severity,
            color: None,
            label: None,
            pulse: false,
            frame: 0,
            theme: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    /// Animation frame (from the runtime tick) driving the pulse.
    pub fn frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for StatusDot<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let color = self.color.unwrap_or(t.severity(self.severity).base);
        let dot = if self.pulse && self.frame % 8 < 4 { "◌" } else { "●" };
        buf.set_string(area.x, area.y, dot, Style::new().fg(color));
        if let Some(label) = self.label {
            if area.width > 2 {
                buf.set_string(
                    area.x + 2,
                    area.y,
                    text::truncate(label, area.width as usize - 2),
                    Style::new().fg(t.fg[1]),
                );
            }
        }
    }
}
