//! The application frame: brand + grouped nav in a sidebar, a topbar row,
//! the content region, and a status bar — the terminal AppShell. The sidebar
//! collapses to a slim rail on narrow terminals (or on Ctrl+B, the toggle
//! the shell's `handle_key` implements).

use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

/// A nav group: optional eyebrow title + links.
#[derive(Clone, Copy, Debug)]
pub struct NavSection<'a> {
    pub title: Option<&'a str>,
    pub items: &'a [&'a str],
}

impl<'a> NavSection<'a> {
    pub fn new(title: Option<&'a str>, items: &'a [&'a str]) -> NavSection<'a> {
        NavSection { title, items }
    }
}

/// Persistent shell state: the active nav item (flattened index across all
/// sections), collapse override, and the measured content region.
#[derive(Clone, Debug, Default)]
pub struct ShellState {
    pub selected: usize,
    /// `None` = auto (collapse under 72 columns).
    pub collapsed: Option<bool>,
    nav_len: usize,
    content: Rect,
    item_rects: Vec<Rect>,
}

impl ShellState {
    pub fn new() -> ShellState {
        ShellState::default()
    }

    /// The content region measured at the last render.
    pub fn content(&self) -> Rect {
        self.content
    }

    /// Click a nav item to activate it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        for (i, rect) in self.item_rects.iter().enumerate() {
            if clicked(ev, *rect) {
                let changed = self.selected != i;
                self.selected = i;
                return if changed { Outcome::Changed } else { Outcome::Consumed };
            }
        }
        Outcome::Ignored
    }

    /// ↑/↓ move the active nav item; Enter submits it; Ctrl+B toggles the
    /// sidebar. Route keys here while nav has focus.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
            self.collapsed = Some(!self.collapsed.unwrap_or(false));
            return Outcome::Changed;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.nav_len > 0 && self.selected < self.nav_len - 1 {
                    self.selected += 1;
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Enter => Outcome::Submitted,
            _ => Outcome::Ignored,
        }
    }
}

/// The shell chrome. Render it first each frame, then render your page into
/// `state.content()`.
#[derive(Clone, Debug)]
pub struct AppShell<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
    sections: &'a [NavSection<'a>],
    topbar: Option<&'a str>,
    topbar_right: Option<&'a str>,
    status: Option<&'a str>,
    status_right: Option<&'a str>,
    nav_focused: bool,
    sidebar_width: u16,
    theme: Option<&'a Theme>,
}

impl<'a> AppShell<'a> {
    pub fn new(title: &'a str, sections: &'a [NavSection<'a>]) -> AppShell<'a> {
        AppShell {
            title,
            subtitle: None,
            sections,
            topbar: None,
            topbar_right: None,
            status: None,
            status_right: None,
            nav_focused: false,
            sidebar_width: 24,
            theme: None,
        }
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Topbar context text (crumbs, page title…).
    pub fn topbar(mut self, topbar: &'a str) -> Self {
        self.topbar = Some(topbar);
        self
    }

    pub fn topbar_right(mut self, right: &'a str) -> Self {
        self.topbar_right = Some(right);
        self
    }

    /// Status bar hints (left side).
    pub fn status(mut self, status: &'a str) -> Self {
        self.status = Some(status);
        self
    }

    pub fn status_right(mut self, right: &'a str) -> Self {
        self.status_right = Some(right);
        self
    }

    pub fn nav_focused(mut self, focused: bool) -> Self {
        self.nav_focused = focused;
        self
    }

    pub fn sidebar_width(mut self, width: u16) -> Self {
        self.sidebar_width = width;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn draw_sidebar(&self, area: Rect, buf: &mut Buffer, t: &Theme, state: &mut ShellState, slim: bool) {
        state.item_rects.clear();
        buf.set_style(area, Style::new().bg(t.bg[1]));
        if area.is_empty() {
            return;
        }
        let bottom = area.y + area.height;
        let mut y = area.y + 1;
        // Brand.
        if y < bottom {
            let brand = if slim { "◆" } else { self.title };
            buf.set_string(
                area.x + 1,
                y,
                text::truncate(brand, area.width.saturating_sub(2) as usize),
                Style::new().fg(t.accent.base).add_modifier(Modifier::BOLD).bg(t.bg[1]),
            );
            y += 1;
        }
        if let (Some(sub), false) = (self.subtitle, slim) {
            if y < bottom {
                buf.set_string(
                    area.x + 1,
                    y,
                    text::truncate(sub, area.width.saturating_sub(2) as usize),
                    Style::new().fg(t.fg[2]).bg(t.bg[1]),
                );
                y += 1;
            }
        }
        y += 1;
        // Sections.
        let mut flat = 0usize;
        for section in self.sections {
            if let (Some(title), false) = (section.title, slim) {
                if y < bottom {
                    let title = title.to_uppercase();
                    buf.set_string(
                        area.x + 1,
                        y,
                        text::truncate(&title, area.width.saturating_sub(2) as usize),
                        Style::new().fg(t.fg[3]).bg(t.bg[1]),
                    );
                    y += 1;
                }
            }
            for item in section.items {
                if y < bottom {
                    state.item_rects.push(Rect::new(area.x, y, area.width, 1));
                    let active = flat == state.selected;
                    if active {
                        buf.set_style(
                            Rect::new(area.x, y, area.width, 1),
                            Style::new().bg(t.bg[3]),
                        );
                        buf.set_string(area.x, y, "▎", Style::new().fg(t.accent.base).bg(t.bg[3]));
                    }
                    let label: std::borrow::Cow<str> = if slim {
                        text::truncate(item, 1)
                    } else {
                        text::truncate(item, area.width.saturating_sub(3) as usize)
                    };
                    let mut style = Style::new()
                        .fg(if active { t.fg[0] } else { t.fg[1] })
                        .bg(if active { t.bg[3] } else { t.bg[1] });
                    if active && self.nav_focused {
                        style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                    }
                    buf.set_string(area.x + 2, y, label, style);
                    y += 1;
                }
                flat += 1;
            }
            if !slim {
                y += 1; // gap between sections
            }
        }
    }
}

impl<'a> StatefulWidget for AppShell<'a> {
    type State = ShellState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ShellState) {
        state.nav_len = self.sections.iter().map(|s| s.items.len()).sum();
        state.selected = state.selected.min(state.nav_len.saturating_sub(1));
        if area.is_empty() {
            state.content = Rect::ZERO;
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        buf.set_style(area, Style::new().bg(t.bg[0]).fg(t.fg[0]));

        let slim = state.collapsed.unwrap_or(area.width < 72);
        let sidebar_w = if slim { 4 } else { self.sidebar_width }.min(area.width);
        let has_status = self.status.is_some() || self.status_right.is_some();
        let status_h: u16 = u16::from(has_status);
        let has_topbar = self.topbar.is_some() || self.topbar_right.is_some();
        let topbar_h: u16 = u16::from(has_topbar);

        let sidebar = Rect::new(area.x, area.y, sidebar_w, area.height - status_h.min(area.height));
        self.draw_sidebar(sidebar, buf, t, state, slim);

        let main_x = area.x + sidebar_w;
        let main_w = area.width.saturating_sub(sidebar_w);
        if has_topbar && area.height > status_h {
            let topbar = Rect::new(main_x, area.y, main_w, 1);
            buf.set_style(topbar, Style::new().bg(t.bg[1]));
            if let Some(text_) = self.topbar {
                buf.set_string(
                    main_x + 2,
                    area.y,
                    text::truncate(text_, main_w.saturating_sub(3) as usize),
                    Style::new().fg(t.fg[1]).bg(t.bg[1]),
                );
            }
            if let Some(right) = self.topbar_right {
                let rw = text::width(right) as u16;
                if main_w > rw + 3 {
                    buf.set_string(
                        main_x + main_w - rw - 1,
                        area.y,
                        right,
                        Style::new().fg(t.fg[2]).bg(t.bg[1]),
                    );
                }
            }
        }
        if has_status {
            let y = area.y + area.height - 1;
            let status = Rect::new(area.x, y, area.width, 1);
            buf.set_style(status, Style::new().bg(t.bg[1]));
            if let Some(left) = self.status {
                buf.set_string(
                    area.x + 1,
                    y,
                    text::truncate(left, area.width.saturating_sub(2) as usize),
                    Style::new().fg(t.fg[2]).bg(t.bg[1]),
                );
            }
            if let Some(right) = self.status_right {
                let rw = text::width(right) as u16;
                if area.width > rw + 2 {
                    buf.set_string(
                        area.x + area.width - rw - 1,
                        y,
                        right,
                        Style::new().fg(t.fg[2]).bg(t.bg[1]),
                    );
                }
            }
        }

        let content_y = area.y + topbar_h;
        let content_h = area
            .height
            .saturating_sub(topbar_h)
            .saturating_sub(status_h);
        state.content = Rect::new(main_x, content_y, main_w, content_h)
            .inner(ratatui::layout::Margin::new(2, 1));
    }
}
