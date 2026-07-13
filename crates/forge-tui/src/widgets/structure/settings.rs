use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

/// Section header for a settings page: eyebrow title + subtle rule.
#[derive(Clone, Debug)]
pub struct SettingsSection<'a> {
    title: &'a str,
    theme: Option<&'a Theme>,
}

impl<'a> SettingsSection<'a> {
    pub fn new(title: &'a str) -> SettingsSection<'a> {
        SettingsSection { title, theme: None }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl Widget for SettingsSection<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let title = self.title.to_uppercase();
        buf.set_string(
            area.x,
            area.y,
            text::truncate(&title, area.width as usize),
            Style::new().fg(t.fg[2]).add_modifier(Modifier::BOLD),
        );
        if area.height >= 2 {
            buf.set_string(
                area.x,
                area.y + 1,
                "─".repeat(area.width as usize),
                Style::new().fg(t.border.subtle),
            );
        }
    }
}

/// One dense settings row: label (and optional help) on the left, the
/// control on the right. Renders the text; render your control into
/// [`SettingsRow::control_area`].
#[derive(Clone, Debug)]
pub struct SettingsRow<'a> {
    label: &'a str,
    help: Option<&'a str>,
    label_width: u16,
    theme: Option<&'a Theme>,
}

impl<'a> SettingsRow<'a> {
    pub fn new(label: &'a str) -> SettingsRow<'a> {
        SettingsRow {
            label,
            help: None,
            label_width: 28,
            theme: None,
        }
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn label_width(mut self, width: u16) -> Self {
        self.label_width = width;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Where the row's control belongs.
    pub fn control_area(&self, area: Rect) -> Rect {
        let lw = self.label_width.min(area.width);
        Rect::new(
            area.x + lw,
            area.y,
            area.width.saturating_sub(lw),
            area.height,
        )
    }
}

impl Widget for SettingsRow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let lw = self.label_width.min(area.width) as usize;
        buf.set_string(
            area.x,
            area.y,
            text::truncate(self.label, lw.saturating_sub(1)),
            Style::new().fg(t.fg[0]),
        );
        if let (Some(help), true) = (self.help, area.height >= 2) {
            buf.set_string(
                area.x,
                area.y + 1,
                text::truncate(help, lw.saturating_sub(1)),
                Style::new().fg(t.fg[2]),
            );
        }
    }
}
