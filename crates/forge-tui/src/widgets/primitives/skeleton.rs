use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Loading placeholder: a dim block with a shimmer band that sweeps on the
/// animation tick (pass the runtime frame counter to animate).
#[derive(Clone, Debug, Default)]
pub struct Skeleton<'a> {
    frame: u64,
    theme: Option<&'a Theme>,
}

impl<'a> Skeleton<'a> {
    pub fn new() -> Skeleton<'a> {
        Skeleton::default()
    }

    /// Animation frame (from the runtime tick).
    pub fn frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Skeleton<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let base = Style::new().fg(t.bg[3]).bg(t.bg[1]);
        let shine = Style::new().fg(t.bg[4]).bg(t.bg[1]);
        let sweep = (self.frame * 2) as i64;
        for dy in 0..area.height {
            for dx in 0..area.width {
                let phase = (dx as i64 + dy as i64 - sweep).rem_euclid(24);
                let (ch, style) = if phase < 4 {
                    ("▒", shine)
                } else {
                    ("░", base)
                };
                buf.set_string(area.x + dx, area.y + dy, ch, style);
            }
        }
    }
}
