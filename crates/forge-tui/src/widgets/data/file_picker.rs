use crate::event::{in_area, is_press, left_down, scroll_delta, Outcome};
use crate::text;
use crate::theme::{Surface, TextRole};
use crate::widgets::paint;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    is_dir: bool,
}

/// Filesystem browser over `std::fs`. Enter descends into directories and
/// submits files (read [`FilePickerState::selected`]); Backspace/← goes up;
/// `.` toggles hidden entries. IO happens on navigation, never per frame.
#[derive(Clone, Debug)]
pub struct FilePickerState {
    cwd: PathBuf,
    entries: Vec<Entry>,
    error: Option<String>,
    pub show_hidden: bool,
    pub cursor: usize,
    offset: usize,
    view_h: usize,
    selected: Option<PathBuf>,
    list_area: Rect,
}

impl FilePickerState {
    pub fn new(dir: impl Into<PathBuf>) -> FilePickerState {
        let mut s = FilePickerState {
            cwd: dir.into(),
            entries: Vec::new(),
            error: None,
            show_hidden: false,
            cursor: 0,
            offset: 0,
            view_h: 0,
            selected: None,
            list_area: Rect::default(),
        };
        s.refresh();
        s
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// The file chosen with Enter, if any (cleared by `take_selected`).
    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    pub fn take_selected(&mut self) -> Option<PathBuf> {
        self.selected.take()
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&self.cwd) {
            Ok(read) => {
                for entry in read.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    self.entries.push(Entry { name, is_dir });
                }
                self.entries.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.cursor = 0;
        self.offset = 0;
    }

    fn enter(&mut self) -> Outcome {
        let Some(entry) = self.entries.get(self.cursor) else {
            return Outcome::Consumed;
        };
        if entry.is_dir {
            self.cwd.push(&entry.name);
            self.refresh();
            Outcome::Changed
        } else {
            self.selected = Some(self.cwd.join(&entry.name));
            Outcome::Submitted
        }
    }

    fn up(&mut self) -> Outcome {
        if self.cwd.pop() {
            self.refresh();
            Outcome::Changed
        } else {
            Outcome::Consumed
        }
    }

    /// Click moves the cursor; clicking the cursor row again activates it
    /// (descend/pick); wheel scrolls.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta != 0 && in_area(ev, self.list_area) {
            self.cursor = if delta < 0 {
                self.cursor.saturating_sub(1)
            } else {
                (self.cursor + 1).min(self.entries.len().saturating_sub(1))
            };
            return Outcome::Consumed;
        }
        if !left_down(ev) || !in_area(ev, self.list_area) {
            return Outcome::Ignored;
        }
        let row = self.offset + (ev.row - self.list_area.y) as usize;
        if row >= self.entries.len() {
            return Outcome::Consumed;
        }
        if row != self.cursor {
            self.cursor = row;
            return Outcome::Consumed;
        }
        self.enter()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let page = self.view_h.max(1);
        match key.code {
            KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.cursor = (self.cursor + 1).min(self.entries.len().saturating_sub(1));
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(page);
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.cursor = (self.cursor + page).min(self.entries.len().saturating_sub(1));
                Outcome::Consumed
            }
            KeyCode::Enter | KeyCode::Right => self.enter(),
            KeyCode::Backspace | KeyCode::Left => self.up(),
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                self.refresh();
                Outcome::Changed
            }
            KeyCode::Esc => Outcome::Cancelled,
            _ => Outcome::Ignored,
        }
    }
}

/// The picker view: breadcrumb path row + directory listing.
#[derive(Clone, Debug, Default)]
pub struct FilePicker {
    focused: bool,
}

impl FilePicker {
    pub fn new() -> FilePicker {
        FilePicker::default()
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl StatefulWidget for FilePicker {
    type State = FilePickerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut FilePickerState) {
        paint(area, |t| {
            let path = state.cwd.display().to_string();
            buf.set_string(
                area.x,
                area.y,
                text::truncate(&path, area.width as usize),
                Style::new().fg(t.text(TextRole::Tertiary)),
            );
            let list = Rect::new(
                area.x,
                area.y + 1,
                area.width,
                area.height.saturating_sub(1),
            );
            state.view_h = list.height as usize;
            state.list_area = list;
            if let Some(err) = &state.error {
                buf.set_string(
                    list.x,
                    list.y,
                    text::truncate(err, list.width as usize),
                    Style::new().fg(t.danger.fg),
                );
                return;
            }
            state.cursor = state.cursor.min(state.entries.len().saturating_sub(1));
            if state.cursor < state.offset {
                state.offset = state.cursor;
            } else if state.view_h > 0 && state.cursor >= state.offset + state.view_h {
                state.offset = state.cursor + 1 - state.view_h;
            }
            for vis in 0..state.view_h {
                let ei = state.offset + vis;
                let Some(entry) = state.entries.get(ei) else {
                    break;
                };
                let y = list.y + vis as u16;
                let is_cursor = ei == state.cursor;
                let mut style = Style::new().fg(if entry.is_dir {
                    t.text(TextRole::Primary)
                } else {
                    t.text(TextRole::Secondary)
                });
                if is_cursor {
                    buf.set_style(
                        Rect::new(list.x, y, list.width, 1),
                        Style::new().bg(t.surface(Surface::Pressed)),
                    );
                    style = style.bg(t.surface(Surface::Pressed));
                    if self.focused {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                }
                let marker = if entry.is_dir { "▸" } else { "·" };
                let mut ms = Style::new().fg(if entry.is_dir {
                    t.accent.base
                } else {
                    t.text(TextRole::Disabled)
                });
                if is_cursor {
                    ms = ms.bg(t.surface(Surface::Pressed));
                }
                buf.set_string(list.x, y, marker, ms);
                buf.set_string(
                    list.x + 2,
                    y,
                    text::truncate(&entry.name, list.width.saturating_sub(2) as usize),
                    style,
                );
            }
        });
    }
}
