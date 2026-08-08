use crate::event::{clicked, is_press, Outcome};
use crate::text;
use crate::theme::{Surface, TextRole};
use crate::widgets::hit::ToggleState;
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;

/// Open/closed state for [`Collapsible`]: the shared click-to-toggle
/// [`ToggleState`] (the header is the clickable rectangle) plus the
/// disclosure arrow keys — → opens, ← closes.
#[derive(Clone, Copy, Debug, Default)]
pub struct CollapsibleState {
    toggle: ToggleState,
}

impl CollapsibleState {
    pub fn new(open: bool) -> CollapsibleState {
        CollapsibleState {
            toggle: ToggleState::new(open),
        }
    }

    /// Is the panel open?
    pub fn open(&self) -> bool {
        self.toggle.on
    }

    pub fn set_open(&mut self, open: bool) {
        self.toggle.on = open;
    }

    /// Click the header to toggle.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        self.toggle.handle_mouse(ev)
    }

    /// Space/Enter toggles; → opens, ← closes.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Right if !self.toggle.on => {
                self.toggle.on = true;
                Outcome::Changed
            }
            KeyCode::Left if self.toggle.on => {
                self.toggle.on = false;
                Outcome::Changed
            }
            _ => self.toggle.handle_key(key),
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
}

impl<'a> Collapsible<'a> {
    pub fn new(title: &'a str) -> Collapsible<'a> {
        Collapsible {
            title,
            body: None,
            focused: false,
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

    /// Rows this widget wants at `width` for the given state.
    pub fn height(&self, width: u16, state: &CollapsibleState) -> u16 {
        if !state.open() {
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
        state.toggle.set_area(Rect::new(area.x, area.y, area.width, 1));
        paint(area, |t| {
            let chevron = if state.open() { "▾" } else { "▸" };
            let mut style = Style::new().fg(t.text(TextRole::Primary));
            if self.focused {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            buf.set_string(
                area.x,
                area.y,
                chevron,
                Style::new().fg(t.text(TextRole::Tertiary)),
            );
            buf.set_string(
                area.x + 2,
                area.y,
                text::truncate(self.title, area.width.saturating_sub(2) as usize),
                style,
            );
            if state.open() {
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
                        buf.set_string(
                            inner.x,
                            y,
                            line,
                            Style::new().fg(t.text(TextRole::Secondary)),
                        );
                    }
                }
            }
        });
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
}

impl<'a> Accordion<'a> {
    pub fn new(items: &'a [(&'a str, &'a str)]) -> Accordion<'a> {
        Accordion {
            items,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'a> StatefulWidget for Accordion<'a> {
    type State = AccordionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut AccordionState) {
        state.len = self.items.len();
        state.headers.clear();
        paint(area, |t| {
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
                let mut style = Style::new().fg(if cursor {
                    t.text(TextRole::Primary)
                } else {
                    t.text(TextRole::Secondary)
                });
                if cursor {
                    if self.focused {
                        style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                    }
                    buf.set_style(
                        Rect::new(area.x, y, area.width, 1),
                        Style::new().bg(t.surface(Surface::Hover)),
                    );
                    style = style.bg(t.surface(Surface::Hover));
                }
                buf.set_string(
                    area.x,
                    y,
                    chevron,
                    Style::new().fg(t.text(TextRole::Tertiary)).bg(if cursor {
                        t.surface(Surface::Hover)
                    } else {
                        t.surface(Surface::Card)
                    }),
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
                        buf.set_string(
                            area.x + 2,
                            y,
                            line,
                            Style::new().fg(t.text(TextRole::Tertiary)),
                        );
                        y += 1;
                    }
                }
            }
        });
    }
}
