use crate::event::{is_press, Outcome};
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;

/// Current page (0-based) and total page count. `pages` is data, so the app
/// sets it directly.
#[derive(Clone, Copy, Debug)]
pub struct PaginationState {
    pub page: usize,
    pub pages: usize,
}

impl PaginationState {
    pub fn new(page: usize, pages: usize) -> PaginationState {
        PaginationState { page, pages }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let last = self.pages.saturating_sub(1);
        let target = match key.code {
            KeyCode::Left => self.page.saturating_sub(1),
            KeyCode::Right => (self.page + 1).min(last),
            KeyCode::Home => 0,
            KeyCode::End => last,
            _ => return Outcome::Ignored,
        };
        if target != self.page {
            self.page = target;
            Outcome::Changed
        } else {
            Outcome::Consumed
        }
    }
}

/// `‹ 1 2 … 7 8 9 … 42 ›` — a window around the current page with first/last
/// anchors, current page on an accent chip.
#[derive(Clone, Debug, Default)]
pub struct Pagination<'a> {
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Pagination<'a> {
    pub fn new() -> Pagination<'a> {
        Pagination::default()
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

fn page_model(page: usize, pages: usize) -> Vec<Option<usize>> {
    // None = ellipsis.
    if pages <= 7 {
        return (0..pages).map(Some).collect();
    }
    let mut out = vec![Some(0)];
    let lo = page.saturating_sub(1).max(1);
    let hi = (page + 1).min(pages - 2);
    if lo > 1 {
        out.push(None);
    }
    for p in lo..=hi {
        out.push(Some(p));
    }
    if hi < pages - 2 {
        out.push(None);
    }
    out.push(Some(pages - 1));
    out
}

impl<'a> StatefulWidget for Pagination<'a> {
    type State = PaginationState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut PaginationState) {
        if area.is_empty() || state.pages == 0 {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let right = area.x + area.width;
        let mut x = area.x;
        let arrow = |enabled: bool| {
            Style::new().fg(if enabled {
                if self.focused { t.accent.fg } else { t.fg[1] }
            } else {
                t.fg[3]
            })
        };
        buf.set_string(x, area.y, "‹", arrow(state.page > 0));
        x += 2;
        for entry in page_model(state.page, state.pages) {
            match entry {
                None => {
                    if x + 2 > right {
                        break;
                    }
                    buf.set_string(x, area.y, "…", Style::new().fg(t.fg[3]));
                    x += 2;
                }
                Some(p) => {
                    let label = (p + 1).to_string();
                    let w = label.len() as u16 + 2;
                    if x + w > right {
                        break;
                    }
                    let style = if p == state.page {
                        Style::new().fg(t.accent.contrast).bg(t.accent.base)
                    } else {
                        Style::new().fg(t.fg[1])
                    };
                    buf.set_style(Rect::new(x, area.y, w, 1), style);
                    buf.set_string(x + 1, area.y, label, style);
                    x += w + 1;
                }
            }
        }
        if x < right {
            buf.set_string(x, area.y, "›", arrow(state.page + 1 < state.pages));
        }
    }
}
