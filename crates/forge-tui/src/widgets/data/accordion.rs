use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug, Default)]
pub struct CollapsibleState {
    pub open: bool,
    header: Rect,
}

impl CollapsibleState {
    pub fn new(open: bool) -> CollapsibleState {
        CollapsibleState {
            open,
            header: Rect::default(),
        }
    }

    /// Click the header to toggle.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if clicked(ev, self.header) {
            self.open = !self.open;
            Outcome::Changed
        } else {
            Outcome::Ignored
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.open = !self.open;
                Outcome::Changed
            }
            KeyCode::Right if !self.open => {
                self.open = true;
                Outcome::Changed
            }
            KeyCode::Left if self.open => {
                self.open = false;
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// `▸ title` header with a wrapped text body when open. For arbitrary
/// content, use [`Collapsible::body_area`] and render it yourself.
#[derive(Clone, Debug)]
pub struct Collapsible<'a> {
    title: &'a str,
    body: Option<&'a str>,
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Collapsible<'a> {
    pub fn new(title: &'a str) -> Collapsible<'a> {
        Collapsible {
            title,
            body: None,
            focused: false,
            theme: None,
        }
    }

    pub fn body(mut self, body: &'a str) -> Self {
        self.body = Some(body);
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Rows this widget wants at `width` for the given state.
    pub fn height(&self, width: u16, state: &CollapsibleState) -> u16 {
        if !state.open {
            return 1;
        }
        1 + self
            .body
            .map(|b| text::wrap(b, width.saturating_sub(2).max(1) as usize).len() as u16)
            .unwrap_or(0)
    }

    /// The content region below the header (open state, custom content).
    pub fn body_area(&self, area: Rect) -> Rect {
        Rect::new(
            area.x + 2,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(1),
        )
    }
}

impl<'a> StatefulWidget for Collapsible<'a> {
    type State = CollapsibleState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut CollapsibleState) {
        state.header = Rect::new(area.x, area.y, area.width, 1);
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let chevron = if state.open { "▾" } else { "▸" };
        let mut style = Style::new().fg(t.fg[0]);
        if self.focused {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        buf.set_string(area.x, area.y, chevron, Style::new().fg(t.fg[2]));
        buf.set_string(
            area.x + 2,
            area.y,
            text::truncate(self.title, area.width.saturating_sub(2) as usize),
            style,
        );
        if state.open {
            if let Some(body) = self.body {
                let inner = self.body_area(area);
                for (i, line) in text::wrap(body, inner.width.max(1) as usize)
                    .iter()
                    .enumerate()
                {
                    let y = inner.y + i as u16;
                    if y >= area.y + area.height {
                        break;
                    }
                    buf.set_string(inner.x, y, line, Style::new().fg(t.fg[1]));
                }
            }
        }
    }
}

/// Exclusive collapsible set: at most one panel open, ↑/↓ moves, Enter
/// toggles.
#[derive(Clone, Debug, Default)]
pub struct AccordionState {
    pub open: Option<usize>,
    pub highlight: usize,
    len: usize,
    headers: Vec<(Rect, usize)>,
}

impl AccordionState {
    pub fn new() -> AccordionState {
        AccordionState::default()
    }

    /// Click a panel header to toggle it.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        for (rect, idx) in self.headers.clone() {
            if clicked(ev, rect) {
                self.highlight = idx;
                self.open = if self.open == Some(idx) {
                    None
                } else {
                    Some(idx)
                };
                return Outcome::Changed;
            }
        }
        Outcome::Ignored
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Up => {
                self.highlight = self.highlight.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                if self.len > 0 && self.highlight + 1 < self.len {
                    self.highlight += 1;
                }
                Outcome::Consumed
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.open = if self.open == Some(self.highlight) {
                    None
                } else {
                    Some(self.highlight)
                };
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// The accordion view over `(title, body)` pairs.
#[derive(Clone, Debug)]
pub struct Accordion<'a> {
    items: &'a [(&'a str, &'a str)],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Accordion<'a> {
    pub fn new(items: &'a [(&'a str, &'a str)]) -> Accordion<'a> {
        Accordion {
            items,
            focused: false,
            theme: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Accordion<'a> {
    type State = AccordionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut AccordionState) {
        state.len = self.items.len();
        state.headers.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let bottom = area.y + area.height;
        let mut y = area.y;
        for (i, (title, body)) in self.items.iter().enumerate() {
            if y >= bottom {
                break;
            }
            state.headers.push((Rect::new(area.x, y, area.width, 1), i));
            let open = state.open == Some(i);
            let cursor = state.highlight == i;
            let chevron = if open { "▾" } else { "▸" };
            let mut style = Style::new().fg(if cursor { t.fg[0] } else { t.fg[1] });
            if cursor {
                if self.focused {
                    style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                }
                buf.set_style(
                    Rect::new(area.x, y, area.width, 1),
                    Style::new().bg(t.bg[2]),
                );
                style = style.bg(t.bg[2]);
            }
            buf.set_string(
                area.x,
                y,
                chevron,
                Style::new()
                    .fg(t.fg[2])
                    .bg(if cursor { t.bg[2] } else { t.bg[1] }),
            );
            buf.set_string(
                area.x + 2,
                y,
                text::truncate(title, area.width.saturating_sub(2) as usize),
                style,
            );
            y += 1;
            if open {
                for line in text::wrap(body, area.width.saturating_sub(2).max(1) as usize) {
                    if y >= bottom {
                        break;
                    }
                    buf.set_string(area.x + 2, y, line, Style::new().fg(t.fg[2]));
                    y += 1;
                }
            }
        }
    }
}
