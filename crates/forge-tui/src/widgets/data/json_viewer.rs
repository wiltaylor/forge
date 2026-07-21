use crate::event::{in_area, is_press, left_down, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::StatefulWidget;
use serde_json::Value;
use std::collections::BTreeSet;

struct Row<'v> {
    path: String,
    depth: usize,
    key: Option<String>,
    value: &'v Value,
    expandable: bool,
}

fn flatten<'v>(
    value: &'v Value,
    key: Option<String>,
    path: String,
    depth: usize,
    expanded: &BTreeSet<String>,
    out: &mut Vec<Row<'v>>,
) {
    let expandable = matches!(value, Value::Object(_) | Value::Array(_));
    let open = expanded.contains(&path);
    out.push(Row {
        path: path.clone(),
        depth,
        key,
        value,
        expandable,
    });
    if !expandable || !open {
        return;
    }
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                flatten(
                    v,
                    Some(k.clone()),
                    format!("{path}.{k}"),
                    depth + 1,
                    expanded,
                    out,
                );
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                flatten(
                    v,
                    Some(format!("[{i}]")),
                    format!("{path}[{i}]"),
                    depth + 1,
                    expanded,
                    out,
                );
            }
        }
        _ => unreachable!(),
    }
}

/// Collapsible, syntax-tinted `serde_json::Value` tree. Root starts
/// expanded. Keys follow the Forge code syntax mapping (properties info,
/// strings success, numbers/atoms info).
#[derive(Clone, Debug)]
pub struct JsonViewerState {
    expanded: BTreeSet<String>,
    pub cursor: usize,
    offset: usize,
    view_h: usize,
    area: Rect,
    /// jq-style path of the cursor row (kept current at render).
    cursor_path: String,
}

impl Default for JsonViewerState {
    fn default() -> JsonViewerState {
        let mut expanded = BTreeSet::new();
        expanded.insert("$".to_string());
        JsonViewerState {
            expanded,
            cursor: 0,
            offset: 0,
            view_h: 0,
            area: Rect::default(),
            cursor_path: "$".into(),
        }
    }
}

impl JsonViewerState {
    pub fn new() -> JsonViewerState {
        JsonViewerState::default()
    }

    /// jq-style path of the cursor row, e.g. `$.nodes[2].name`.
    pub fn cursor_path(&self) -> &str {
        &self.cursor_path
    }

    /// Click moves the cursor; clicking the cursor row toggles expansion;
    /// wheel scrolls.
    pub fn handle_mouse(&mut self, ev: &MouseEvent, value: &Value) -> Outcome {
        let delta = scroll_delta(ev);
        if delta != 0 && in_area(ev, self.area) {
            self.cursor = if delta < 0 {
                self.cursor.saturating_sub(1)
            } else {
                self.cursor + 1
            };
            return Outcome::Consumed;
        }
        if !left_down(ev) || !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        let mut rows = Vec::new();
        flatten(value, None, "$".into(), 0, &self.expanded, &mut rows);
        let row = self.offset + (ev.row - self.area.y) as usize;
        if row >= rows.len() {
            return Outcome::Consumed;
        }
        if row != self.cursor {
            self.cursor = row;
            return Outcome::Consumed;
        }
        let r = &rows[row];
        if r.expandable {
            if !self.expanded.remove(&r.path) {
                self.expanded.insert(r.path.clone());
            }
            return Outcome::Changed;
        }
        Outcome::Consumed
    }

    pub fn handle_key(&mut self, key: KeyEvent, value: &Value) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let mut rows = Vec::new();
        flatten(value, None, "$".into(), 0, &self.expanded, &mut rows);
        if rows.is_empty() {
            return Outcome::Ignored;
        }
        self.cursor = self.cursor.min(rows.len() - 1);
        let row = &rows[self.cursor];
        match key.code {
            KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.cursor = (self.cursor + 1).min(rows.len() - 1);
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(self.view_h.max(1));
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.cursor = (self.cursor + self.view_h.max(1)).min(rows.len() - 1);
                Outcome::Consumed
            }
            KeyCode::Right | KeyCode::Enter if row.expandable => {
                if self.expanded.insert(row.path.clone()) {
                    Outcome::Changed
                } else {
                    self.expanded.remove(&row.path);
                    Outcome::Changed
                }
            }
            KeyCode::Left => {
                if row.expandable && self.expanded.remove(&row.path) {
                    Outcome::Changed
                } else {
                    Outcome::Consumed
                }
            }
            _ => Outcome::Ignored,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct JsonViewer<'a> {
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> JsonViewer<'a> {
    pub fn new() -> JsonViewer<'a> {
        JsonViewer::default()
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

fn preview(value: &Value, open: bool) -> String {
    match value {
        Value::Object(_) if open => "{".into(),
        Value::Object(m) => format!("{{…}} {} keys", m.len()),
        Value::Array(_) if open => "[".into(),
        Value::Array(a) => format!("[…] {} items", a.len()),
        Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

/// Render a JSON tree. The `value` is passed at render time (widget borrows
/// nothing between frames).
impl<'a> JsonViewer<'a> {
    pub fn render_value(
        self,
        value: &Value,
        area: Rect,
        buf: &mut Buffer,
        state: &mut JsonViewerState,
    ) {
        StatefulWidget::render(ValueViewer { inner: self, value }, area, buf, state);
    }
}

struct ValueViewer<'a, 'v> {
    inner: JsonViewer<'a>,
    value: &'v Value,
}

impl StatefulWidget for ValueViewer<'_, '_> {
    type State = JsonViewerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut JsonViewerState) {
        state.view_h = area.height as usize;
        state.area = area;
        if area.is_empty() {
            return;
        }
        let t = self.inner.theme.unwrap_or_else(|| default_theme());
        let mut rows = Vec::new();
        flatten(self.value, None, "$".into(), 0, &state.expanded, &mut rows);
        if rows.is_empty() {
            return;
        }
        state.cursor = state.cursor.min(rows.len() - 1);
        state.cursor_path = rows[state.cursor].path.clone();
        if state.cursor < state.offset {
            state.offset = state.cursor;
        } else if state.cursor >= state.offset + state.view_h {
            state.offset = state.cursor + 1 - state.view_h;
        }
        for vis in 0..state.view_h {
            let ri = state.offset + vis;
            let Some(row) = rows.get(ri) else { break };
            let y = area.y + vis as u16;
            let is_cursor = ri == state.cursor;
            if is_cursor {
                let mut s = Style::new().bg(t.bg[3]);
                if self.inner.focused {
                    s = s.add_modifier(Modifier::BOLD);
                }
                buf.set_style(Rect::new(area.x, y, area.width, 1), s);
            }
            let bg = if is_cursor { Some(t.bg[3]) } else { None };
            let paint = |style: Style| match bg {
                Some(b) => style.bg(b),
                None => style,
            };
            let indent = (row.depth * 2) as u16;
            let mut x = area.x + indent;
            let right = area.x + area.width;
            if x + 2 > right {
                continue;
            }
            let open = state.expanded.contains(&row.path);
            let marker = if !row.expandable {
                " "
            } else if open {
                "▾"
            } else {
                "▸"
            };
            buf.set_string(x, y, marker, paint(Style::new().fg(t.fg[2])));
            x += 2;
            if let Some(key) = &row.key {
                let k = text::truncate(key, (right - x) as usize);
                buf.set_string(x, y, &k, paint(Style::new().fg(t.info.fg)));
                x += text::width(&k) as u16;
                if x + 2 <= right {
                    buf.set_string(x, y, ": ", paint(Style::new().fg(t.fg[2])));
                    x += 2;
                }
            }
            if x < right {
                let pv = preview(row.value, open);
                let color = match row.value {
                    Value::String(_) => t.success.fg,
                    Value::Number(_) | Value::Bool(_) | Value::Null => t.info.fg,
                    _ => t.fg[2],
                };
                buf.set_string(
                    x,
                    y,
                    text::truncate(&pv, (right - x) as usize),
                    paint(Style::new().fg(color)),
                );
            }
        }
    }
}
