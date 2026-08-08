//! Collapsible, syntax-tinted `serde_json::Value` tree. Expansion is keyed
//! by JSON Pointer paths (`""` = root, `"/nodes/2/name"`), so it survives
//! value updates. The root starts expanded.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{Surface, TextRole, Theme};
use crate::widgets::util;
use egui::{Color32, FontId, Response, Sense, Ui, Vec2, WidgetInfo, WidgetType};
use serde_json::Value;
use std::collections::HashSet;

/// Set of expanded JSON Pointer paths.
#[derive(Clone, Debug)]
pub struct JsonViewerState {
    pub expanded: HashSet<String>,
}

impl Default for JsonViewerState {
    fn default() -> JsonViewerState {
        let mut expanded = HashSet::new();
        expanded.insert(String::new()); // root open
        JsonViewerState { expanded }
    }
}

impl JsonViewerState {
    pub fn new() -> JsonViewerState {
        JsonViewerState::default()
    }

    pub fn is_expanded(&self, path: &str) -> bool {
        self.expanded.contains(path)
    }

    /// Toggle one path; returns the new expanded-ness.
    pub fn toggle(&mut self, path: &str) -> bool {
        if self.expanded.remove(path) {
            false
        } else {
            self.expanded.insert(path.to_owned());
            true
        }
    }
}

/// JSON Pointer path of a child: `~`/`/` escaped per RFC 6901.
pub(crate) fn child_path(parent: &str, key: &str) -> String {
    let escaped = key.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

const INDENT: f32 = 16.0;

/// The viewer widget.
pub struct JsonViewer<'a> {
    state: &'a mut JsonViewerState,
    value: &'a Value,
}

impl<'a> JsonViewer<'a> {
    pub fn new(state: &'a mut JsonViewerState, value: &'a Value) -> JsonViewer<'a> {
        JsonViewer { state, value }
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self { state, value } = self;
        let mut painter = RowPainter {
            t: &t,
            font: t.mono(t.type_scale.sm),
            outcome: Outcome::Ignored,
            union: None,
        };
        ui.spacing_mut().item_spacing.y = 0.0;
        painter.value_ui(ui, state, value, None, String::new(), 0);
        let RowPainter { outcome, union, .. } = painter;
        let response = union.unwrap_or_else(|| {
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 0.0), Sense::hover())
                .1
        });
        ForgeResponse::new(response, outcome)
    }
}

struct RowPainter<'t> {
    t: &'t Theme,
    font: FontId,
    outcome: Outcome,
    union: Option<Response>,
}

/// One text run of a row.
struct Seg<'s>(&'s str, Color32);

impl RowPainter<'_> {
    fn value_ui(
        &mut self,
        ui: &mut Ui,
        state: &mut JsonViewerState,
        value: &Value,
        key: Option<&str>,
        path: String,
        depth: usize,
    ) {
        let t = self.t;
        let expandable = matches!(value, Value::Object(_) | Value::Array(_));
        let open = expandable && state.is_expanded(&path);

        // The row: [disclosure] [key:] value-or-preview
        let key_owned = key.map(|k| format!("\"{k}\""));
        let mut segs: Vec<Seg> = Vec::new();
        if expandable {
            segs.push(Seg(
                if open { "▾ " } else { "▸ " },
                t.text(TextRole::Disabled),
            ));
        } else {
            segs.push(Seg("  ", t.text(TextRole::Disabled)));
        }
        if let Some(k) = &key_owned {
            segs.push(Seg(k, t.accent.fg));
            segs.push(Seg(": ", t.text(TextRole::Disabled)));
        }
        let preview = match value {
            Value::Object(_) if open => "{".to_owned(),
            Value::Object(m) => format!(
                "{{…}} {} {}",
                m.len(),
                if m.len() == 1 { "key" } else { "keys" }
            ),
            Value::Array(_) if open => "[".to_owned(),
            Value::Array(a) => format!(
                "[…] {} {}",
                a.len(),
                if a.len() == 1 { "item" } else { "items" }
            ),
            Value::String(s) => format!("\"{s}\""),
            other => other.to_string(),
        };
        let color = match value {
            Value::Object(_) | Value::Array(_) => t.text(TextRole::Disabled),
            Value::String(_) => t.success.base,
            Value::Number(_) => t.warning.base,
            Value::Bool(_) | Value::Null => t.info.base,
        };
        segs.push(Seg(&preview, color));

        let resp = self.row(ui, &segs, depth, expandable, &path);
        if expandable && resp.clicked() {
            state.toggle(&path);
            self.outcome = self.outcome.merge(Outcome::Changed);
        }
        let open = expandable && state.is_expanded(&path);
        self.merge_union(resp);

        if open {
            match value {
                Value::Object(map) => {
                    for (k, v) in map {
                        let cp = child_path(&path, k);
                        self.value_ui(ui, state, v, Some(k), cp, depth + 1);
                    }
                    let close = [
                        Seg("  ", t.text(TextRole::Disabled)),
                        Seg("}", t.text(TextRole::Disabled)),
                    ];
                    let r = self.row(ui, &close, depth, false, &path);
                    self.merge_union(r);
                }
                Value::Array(items) => {
                    for (i, v) in items.iter().enumerate() {
                        let key = i.to_string();
                        let cp = child_path(&path, &key);
                        self.value_ui(ui, state, v, None, cp, depth + 1);
                    }
                    let close = [
                        Seg("  ", t.text(TextRole::Disabled)),
                        Seg("]", t.text(TextRole::Disabled)),
                    ];
                    let r = self.row(ui, &close, depth, false, &path);
                    self.merge_union(r);
                }
                _ => unreachable!(),
            }
        }
    }

    fn row(
        &self,
        ui: &mut Ui,
        segs: &[Seg],
        depth: usize,
        clickable: bool,
        path: &str,
    ) -> Response {
        let t = self.t;
        let row_h = t.type_scale.sm + 8.0;
        let sense = if clickable {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), sense);
        if clickable {
            let label = if path.is_empty() { "$" } else { path };
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
        }
        if ui.is_rect_visible(rect) {
            if clickable && resp.hovered() {
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(t.radius.sm as u8),
                    t.surface(Surface::Hover),
                );
            }
            let mut x = rect.min.x + depth as f32 * INDENT + 4.0;
            let cy = rect.center().y;
            let clip = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
            for Seg(text, color) in segs {
                let g = util::galley(ui, *text, self.font.clone(), *color);
                let w = g.size().x;
                clip.galley(egui::pos2(x, cy - g.size().y / 2.0), g, *color);
                x += w;
            }
        }
        resp
    }

    fn merge_union(&mut self, resp: Response) {
        self.union = Some(match self.union.take() {
            Some(u) => u.union(resp),
            None => resp,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_path_builds_json_pointers() {
        assert_eq!(child_path("", "name"), "/name");
        assert_eq!(child_path("/nodes", "2"), "/nodes/2");
        assert_eq!(child_path("", "a/b"), "/a~1b");
        assert_eq!(child_path("", "t~e"), "/t~0e");
    }

    #[test]
    fn root_expanded_by_default_and_toggle_flips() {
        let mut s = JsonViewerState::default();
        assert!(s.is_expanded(""));
        assert!(!s.is_expanded("/nodes"));
        assert!(s.toggle("/nodes"));
        assert!(s.is_expanded("/nodes"));
        assert!(!s.toggle("/nodes"));
        assert!(!s.is_expanded("/nodes"));
        // Root collapses too.
        assert!(!s.toggle(""));
        assert!(!s.is_expanded(""));
    }
}
