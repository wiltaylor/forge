use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Braille spinner driven by the runtime frame counter.
#[derive(Clone, Debug, Default)]
pub struct Spinner<'a> {
    frame: u64,
    label: Option<&'a str>,
    color: Option<Color>,
    theme: Option<&'a Theme>,
}

impl<'a> Spinner<'a> {
    pub fn new() -> Spinner<'a> {
        Spinner::default()
    }

    /// Animation frame (from the runtime tick).
    pub fn frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Spinner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let color = self.color.unwrap_or(t.accent.base);
        let glyph = FRAMES[(self.frame as usize) % FRAMES.len()];
        buf.set_string(area.x, area.y, glyph, Style::new().fg(color));
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
