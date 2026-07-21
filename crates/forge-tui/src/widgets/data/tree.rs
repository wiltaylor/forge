use crate::event::{in_area, is_press, left_down, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;
use std::collections::BTreeSet;

/// A borrowed tree node (build these per frame from your data).
#[derive(Clone, Copy, Debug)]
pub struct TreeNode<'a> {
    pub label: &'a str,
    pub children: &'a [TreeNode<'a>],
}

impl<'a> TreeNode<'a> {
    pub fn leaf(label: &'a str) -> TreeNode<'a> {
        TreeNode {
            label,
            children: &[],
        }
    }

    pub fn branch(label: &'a str, children: &'a [TreeNode<'a>]) -> TreeNode<'a> {
        TreeNode { label, children }
    }
}

fn flatten<'a>(
    nodes: &'a [TreeNode<'a>],
    expanded: &BTreeSet<Vec<usize>>,
    path: &mut Vec<usize>,
    out: &mut Vec<(Vec<usize>, usize, TreeNode<'a>)>,
) {
    for (i, node) in nodes.iter().enumerate() {
        path.push(i);
        out.push((path.clone(), path.len() - 1, *node));
        if !node.children.is_empty() && expanded.contains(path) {
            flatten(node.children, expanded, path, out);
        }
        path.pop();
    }
}

/// Expansion set + cursor over the visible rows. Key handling needs the
/// roots (the tree shape lives in your data): `handle_key(key, roots)`.
#[derive(Clone, Debug, Default)]
pub struct TreeState {
    expanded: BTreeSet<Vec<usize>>,
    pub cursor: usize,
    offset: usize,
    view_h: usize,
    area: Rect,
}

impl TreeState {
    pub fn new() -> TreeState {
        TreeState::default()
    }

    pub fn expand(&mut self, path: &[usize]) {
        self.expanded.insert(path.to_vec());
    }

    pub fn collapse(&mut self, path: &[usize]) {
        self.expanded.remove(path);
    }

    pub fn is_expanded(&self, path: &[usize]) -> bool {
        self.expanded.contains(path)
    }

    /// Path of the cursor row within the current tree.
    pub fn cursor_path(&self, roots: &[TreeNode]) -> Option<Vec<usize>> {
        let mut rows = Vec::new();
        flatten(roots, &self.expanded, &mut Vec::new(), &mut rows);
        rows.get(self.cursor).map(|(p, _, _)| p.clone())
    }

    /// Click moves the cursor; clicking the cursor row again toggles a
    /// branch (or submits a leaf); wheel scrolls.
    pub fn handle_mouse(&mut self, ev: &MouseEvent, roots: &[TreeNode]) -> Outcome {
        let delta = scroll_delta(ev);
        if delta != 0 && in_area(ev, self.area) {
            self.cursor = if delta < 0 {
                self.cursor.saturating_sub(1)
            } else {
                self.cursor + 1 // clamped at render/flatten below
            };
            return Outcome::Consumed;
        }
        if !left_down(ev) || !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        let mut rows = Vec::new();
        flatten(roots, &self.expanded, &mut Vec::new(), &mut rows);
        if rows.is_empty() {
            return Outcome::Ignored;
        }
        let row = self.offset + (ev.row - self.area.y) as usize;
        if row >= rows.len() {
            return Outcome::Consumed;
        }
        if row != self.cursor {
            self.cursor = row;
            return Outcome::Consumed;
        }
        // Second click on the cursor row: toggle branch / submit leaf.
        let (path, _, node) = &rows[row];
        if node.children.is_empty() {
            Outcome::Submitted
        } else if self.expanded.remove(path) {
            Outcome::Changed
        } else {
            self.expanded.insert(path.clone());
            Outcome::Changed
        }
    }

    /// ↑/↓ move; → expands (or steps into), ← collapses (or steps to the
    /// parent); Enter submits the cursor row.
    pub fn handle_key(&mut self, key: KeyEvent, roots: &[TreeNode]) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let mut rows = Vec::new();
        flatten(roots, &self.expanded, &mut Vec::new(), &mut rows);
        if rows.is_empty() {
            return Outcome::Ignored;
        }
        self.cursor = self.cursor.min(rows.len() - 1);
        let (path, _, node) = &rows[self.cursor];
        match key.code {
            KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.cursor = (self.cursor + 1).min(rows.len() - 1);
                Outcome::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                Outcome::Consumed
            }
            KeyCode::End => {
                self.cursor = rows.len() - 1;
                Outcome::Consumed
            }
            KeyCode::Right => {
                if !node.children.is_empty() && !self.expanded.contains(path) {
                    self.expanded.insert(path.clone());
                    Outcome::Changed
                } else if !node.children.is_empty() {
                    self.cursor += 1; // step into the first child
                    Outcome::Consumed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Left => {
                if self.expanded.contains(path) {
                    self.expanded.remove(path);
                    Outcome::Changed
                } else if path.len() > 1 {
                    // Jump to the parent row.
                    let parent = &path[..path.len() - 1];
                    if let Some(pi) = rows.iter().position(|(p, _, _)| p == parent) {
                        self.cursor = pi;
                    }
                    Outcome::Consumed
                } else {
                    Outcome::Consumed
                }
            }
            KeyCode::Enter => Outcome::Submitted,
            _ => Outcome::Ignored,
        }
    }
}

/// Hierarchical expandable list: `▸ branch / · leaf`, indented per depth.
#[derive(Clone, Debug)]
pub struct Tree<'a> {
    roots: &'a [TreeNode<'a>],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Tree<'a> {
    pub fn new(roots: &'a [TreeNode<'a>]) -> Tree<'a> {
        Tree {
            roots,
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

impl<'a> StatefulWidget for Tree<'a> {
    type State = TreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TreeState) {
        state.view_h = area.height as usize;
        state.area = area;
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let mut rows = Vec::new();
        flatten(self.roots, &state.expanded, &mut Vec::new(), &mut rows);
        if rows.is_empty() {
            return;
        }
        state.cursor = state.cursor.min(rows.len() - 1);
        if state.cursor < state.offset {
            state.offset = state.cursor;
        } else if state.cursor >= state.offset + state.view_h {
            state.offset = state.cursor + 1 - state.view_h;
        }
        for vis in 0..state.view_h {
            let ri = state.offset + vis;
            let Some((path, depth, node)) = rows.get(ri) else {
                break;
            };
            let y = area.y + vis as u16;
            let is_cursor = ri == state.cursor;
            let indent = (depth * 2) as u16;
            let mut style = Style::new().fg(if is_cursor { t.fg[0] } else { t.fg[1] });
            if is_cursor {
                buf.set_style(
                    Rect::new(area.x, y, area.width, 1),
                    Style::new().bg(t.bg[3]),
                );
                style = style.bg(t.bg[3]);
                if self.focused {
                    style = style.add_modifier(Modifier::BOLD);
                }
            }
            let marker = if node.children.is_empty() {
                "·"
            } else if state.expanded.contains(path) {
                "▾"
            } else {
                "▸"
            };
            if indent + 2 < area.width {
                let mut marker_style = Style::new().fg(t.fg[2]);
                if is_cursor {
                    marker_style = marker_style.bg(t.bg[3]);
                }
                buf.set_string(area.x + indent, y, marker, marker_style);
                buf.set_string(
                    area.x + indent + 2,
                    y,
                    text::truncate(node.label, (area.width - indent - 2) as usize),
                    style,
                );
            }
        }
    }
}
