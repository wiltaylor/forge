use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

/// KPI tile: eyebrow label, big value, optional delta with trend arrow.
/// Wants three rows; degrades to whatever height it gets.
#[derive(Clone, Debug)]
pub struct Stat<'a> {
    label: &'a str,
    value: &'a str,
    delta: Option<(&'a str, Trend)>,
    /// Whether an upward trend is good (colors the delta success/danger).
    up_is_good: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Stat<'a> {
    pub fn new(label: &'a str, value: &'a str) -> Stat<'a> {
        Stat { label, value, delta: None, up_is_good: true, theme: None }
    }

    pub fn delta(mut self, delta: &'a str, trend: Trend) -> Self {
        self.delta = Some((delta, trend));
        self
    }

    pub fn up_is_good(mut self, up_is_good: bool) -> Self {
        self.up_is_good = up_is_good;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for Stat<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let w = area.width as usize;
        let label = self.label.to_uppercase();
        buf.set_string(
            area.x,
            area.y,
            text::truncate(&label, w),
            Style::new().fg(t.fg[2]),
        );
        if area.height >= 2 {
            buf.set_string(
                area.x,
                area.y + 1,
                text::truncate(self.value, w),
                Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD),
            );
        }
        if area.height >= 3 {
            if let Some((delta, trend)) = self.delta {
                let (arrow, good) = match trend {
                    Trend::Up => ("↑", self.up_is_good),
                    Trend::Down => ("↓", !self.up_is_good),
                    Trend::Flat => ("→", true),
                };
                let color = if trend == Trend::Flat {
                    t.fg[2]
                } else if good {
                    t.success.base
                } else {
                    t.danger.base
                };
                let line = format!("{arrow} {delta}");
                buf.set_string(
                    area.x,
                    area.y + 2,
                    text::truncate(&line, w),
                    Style::new().fg(color),
                );
            }
        }
    }
}
