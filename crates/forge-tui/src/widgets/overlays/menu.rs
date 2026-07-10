use crate::event::{in_area, is_press, left_down, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use crate::widgets::overlays::place;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Clear, StatefulWidget, Widget};

/// One row of a menu.
#[derive(Clone, Debug)]
pub enum MenuEntry<'a> {
    Item {
        label: &'a str,
        kbd: Option<&'a str>,
        danger: bool,
        disabled: bool,
    },
    Section(&'a str),
    Separator,
}

impl<'a> MenuEntry<'a> {
    pub fn item(label: &'a str) -> MenuEntry<'a> {
        MenuEntry::Item { label, kbd: None, danger: false, disabled: false }
    }

    pub fn item_kbd(label: &'a str, kbd: &'a str) -> MenuEntry<'a> {
        MenuEntry::Item { label, kbd: Some(kbd), danger: false, disabled: false }
    }

    pub fn danger(label: &'a str) -> MenuEntry<'a> {
        MenuEntry::Item { label, kbd: None, danger: true, disabled: false }
    }

    fn selectable(&self) -> bool {
        matches!(self, MenuEntry::Item { disabled: false, .. })
    }
}

/// Cursor over the *selectable* items of a menu (sections/separators are
/// skipped). `highlight` indexes selectable items in order.
#[derive(Clone, Debug, Default)]
pub struct MenuState {
    pub highlight: usize,
    len: usize,
    panel: Rect,
    item_rects: Vec<(Rect, usize)>,
}

impl MenuState {
    pub fn new() -> MenuState {
        MenuState::default()
    }

    /// Hover moves the cursor; click submits the item under the pointer;
    /// clicking outside the panel cancels.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if matches!(ev.kind, MouseEventKind::Moved) && in_area(ev, self.panel) {
            for (rect, idx) in &self.item_rects {
                if in_area(ev, *rect) {
                    self.highlight = *idx;
                    return Outcome::Consumed;
                }
            }
            return Outcome::Consumed;
        }
        if !left_down(ev) {
            return Outcome::Ignored;
        }
        if !in_area(ev, self.panel) {
            return Outcome::Cancelled; // click-away
        }
        for (rect, idx) in &self.item_rects {
            if in_area(ev, *rect) {
                self.highlight = *idx;
                return Outcome::Submitted;
            }
        }
        Outcome::Consumed
    }

    /// ↑/↓ move; Enter submits (read `highlight`); Esc cancels.
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
            KeyCode::Home => {
                self.highlight = 0;
                Outcome::Consumed
            }
            KeyCode::End => {
                self.highlight = self.len.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Enter => Outcome::Submitted,
            KeyCode::Esc => Outcome::Cancelled,
            _ => Outcome::Ignored,
        }
    }

    /// Jump to the next selectable item starting with `c`; submits when it
    /// lands (menu mnemonic behavior).
    pub fn mnemonic(&mut self, entries: &[MenuEntry], c: char) -> Outcome {
        let labels: Vec<&str> = entries
            .iter()
            .filter_map(|e| match e {
                MenuEntry::Item { label, disabled: false, .. } => Some(*label),
                _ => None,
            })
            .collect();
        let lc = c.to_ascii_lowercase();
        let n = labels.len();
        for step in 1..=n {
            let i = (self.highlight + step) % n;
            if labels[i].to_lowercase().starts_with(lc) {
                self.highlight = i;
                return Outcome::Submitted;
            }
        }
        Outcome::Ignored
    }
}

/// Anchored action menu (dropdown / context menu — same widget, the anchor
/// is a button rect or the mouse position). Floats on the popover surface.
#[derive(Clone, Debug)]
pub struct DropdownMenu<'a> {
    entries: &'a [MenuEntry<'a>],
    anchor: Rect,
    theme: Option<&'a Theme>,
}

impl<'a> DropdownMenu<'a> {
    pub fn new(entries: &'a [MenuEntry<'a>], anchor: Rect) -> DropdownMenu<'a> {
        DropdownMenu { entries, anchor, theme: None }
    }

    /// Context-menu constructor: anchor at a point (e.g. the mouse).
    pub fn at(entries: &'a [MenuEntry<'a>], x: u16, y: u16) -> DropdownMenu<'a> {
        DropdownMenu::new(entries, Rect::new(x, y, 1, 1))
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Natural panel size.
    pub fn size(&self) -> (u16, u16) {
        let w = self
            .entries
            .iter()
            .map(|e| match e {
                MenuEntry::Item { label, kbd, .. } => {
                    text::width(label) + kbd.map(|k| text::width(k) + 3).unwrap_or(0)
                }
                MenuEntry::Section(s) => text::width(s),
                MenuEntry::Separator => 0,
            })
            .max()
            .unwrap_or(0) as u16
            + 4;
        (w.max(16), self.entries.len() as u16 + 2)
    }
}

impl<'a> StatefulWidget for DropdownMenu<'a> {
    type State = MenuState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut MenuState) {
        state.len = self.entries.iter().filter(|e| e.selectable()).count();
        state.item_rects.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let panel = place(self.anchor, self.size(), area);
        state.panel = panel;
        Clear.render(panel, buf);
        let block = ratatui::widgets::Block::bordered()
            .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
            .style(Style::new().bg(t.bg[4]));
        let inner = block.inner(panel);
        block.render(panel, buf);

        let mut selectable_idx = 0usize;
        for (row, entry) in self.entries.iter().enumerate() {
            let y = inner.y + row as u16;
            if y >= inner.y + inner.height {
                break;
            }
            match entry {
                MenuEntry::Separator => {
                    buf.set_string(
                        inner.x,
                        y,
                        "─".repeat(inner.width as usize),
                        Style::new().fg(t.border.subtle).bg(t.bg[4]),
                    );
                }
                MenuEntry::Section(title) => {
                    let title = title.to_uppercase();
                    buf.set_string(
                        inner.x + 1,
                        y,
                        text::truncate(&title, inner.width.saturating_sub(2) as usize),
                        Style::new().fg(t.fg[2]).bg(t.bg[4]),
                    );
                }
                MenuEntry::Item { label, kbd, danger, disabled } => {
                    if entry.selectable() {
                        state.item_rects.push((Rect::new(inner.x, y, inner.width, 1), selectable_idx));
                    }
                    let is_cursor = entry.selectable() && selectable_idx == state.highlight;
                    let mut style = Style::new()
                        .fg(if *disabled {
                            t.fg[3]
                        } else if *danger {
                            t.danger.fg
                        } else {
                            t.fg[0]
                        })
                        .bg(t.bg[4]);
                    if is_cursor {
                        style = style.bg(t.bg[3]).add_modifier(Modifier::BOLD);
                        buf.set_style(Rect::new(inner.x, y, inner.width, 1), style);
                    }
                    buf.set_string(
                        inner.x + 1,
                        y,
                        text::truncate(label, inner.width.saturating_sub(2) as usize),
                        style,
                    );
                    if let Some(kbd) = kbd {
                        let kw = text::width(kbd) as u16;
                        if inner.width > kw + 2 {
                            buf.set_string(
                                inner.x + inner.width - kw - 1,
                                y,
                                *kbd,
                                Style::new().fg(t.fg[2]).bg(if is_cursor { t.bg[3] } else { t.bg[4] }),
                            );
                        }
                    }
                    if entry.selectable() {
                        selectable_idx += 1;
                    }
                }
            }
        }
    }
}
