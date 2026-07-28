//! Follow-mode log stream: mono rows, severity badges, live substring
//! filter. `pinned` sticks the view to the tail; scrolling up unpins, the
//! "Follow" chip (or scrolling back to the bottom) re-pins.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::util;
use crate::widgets::Tone;
use egui::{CornerRadius, Rect, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

/// Log severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Debug => "DBG",
            Level::Info => "INF",
            Level::Warn => "WRN",
            Level::Error => "ERR",
        }
    }

    fn tone(self) -> Tone {
        match self {
            Level::Debug => Tone::Neutral,
            Level::Info => Tone::Info,
            Level::Warn => Tone::Warning,
            Level::Error => Tone::Danger,
        }
    }
}

/// One log record. `time` is preformatted — Logs never parses it.
#[derive(Clone, Debug)]
pub struct LogLine {
    pub time: String,
    pub level: Level,
    pub message: String,
}

impl LogLine {
    pub fn new(time: impl Into<String>, level: Level, message: impl Into<String>) -> LogLine {
        LogLine {
            time: time.into(),
            level,
            message: message.into(),
        }
    }
}

/// Follow flag + filter text. Plain app-owned data.
#[derive(Clone, Debug)]
pub struct LogsState {
    /// Stick the view to the newest line. Cleared when the user scrolls up.
    pub pinned: bool,
    /// Case-insensitive substring filter on the message.
    pub filter: String,
}

impl Default for LogsState {
    fn default() -> LogsState {
        LogsState {
            pinned: true,
            filter: String::new(),
        }
    }
}

pub(crate) fn line_matches(line: &LogLine, filter: &str) -> bool {
    filter.is_empty() || line.message.to_lowercase().contains(&filter.to_lowercase())
}

/// The log viewer widget.
pub struct Logs<'a> {
    state: &'a mut LogsState,
    lines: &'a [LogLine],
    height: f32,
}

impl<'a> Logs<'a> {
    pub fn new(state: &'a mut LogsState, lines: &'a [LogLine]) -> Logs<'a> {
        Logs {
            state,
            lines,
            height: 240.0,
        }
    }

    /// Scroll viewport height (default 240).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let Self {
            state,
            lines,
            height,
        } = self;
        let mut outcome = Outcome::Ignored;

        // ---- Header: filter input + follow chip.
        ui.horizontal(|ui| {
            let r = crate::widgets::forms::Input::new(&mut state.filter)
                .placeholder("Filter…")
                .icon(crate::widgets::primitives::Glyph::Search)
                .desired_width(200.0)
                .show(ui);
            outcome = outcome.merge(r.outcome);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let chip = follow_chip(ui, &t, state.pinned);
                if chip.clicked() {
                    state.pinned = !state.pinned;
                    outcome = outcome.merge(Outcome::Changed);
                }
            });
        });
        ui.add_space(t.space.x(1.5));

        // ---- The stream.
        let mono = t.mono(t.type_scale.sm);
        let badge_font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.xs);
        let row_h = t.type_scale.sm + 8.0;
        let visible: Vec<&LogLine> = lines
            .iter()
            .filter(|l| line_matches(l, &state.filter))
            .collect();

        let frame = egui::Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(egui::Margin::same(4));
        let inner = frame.show(ui, |ui| {
            // show_rows spaces rows by item_spacing when computing offsets —
            // zero it here so the virtual math matches the painted rows.
            ui.spacing_mut().item_spacing.y = 0.0;
            let out = egui::ScrollArea::vertical()
                .id_salt(ui.id().with("logs"))
                .max_height(height)
                .min_scrolled_height(height)
                .auto_shrink([false, false])
                .stick_to_bottom(state.pinned)
                .show_rows(ui, row_h, visible.len(), |ui, range| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for li in range {
                        let line = visible[li];
                        let (rect, _r) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), row_h),
                            Sense::hover(),
                        );
                        if !ui.is_rect_visible(rect) {
                            continue;
                        }
                        let mut x = rect.min.x + 4.0;
                        let cy = rect.center().y;
                        // time — fg[3]
                        let g = util::galley(ui, &line.time, mono.clone(), t.fg[3]);
                        ui.painter()
                            .galley(egui::pos2(x, cy - g.size().y / 2.0), g, t.fg[3]);
                        x += 74.0;
                        // level badge
                        let (base, bg, _fg) = line.level.tone().triple(&t);
                        let g = util::galley(ui, line.level.label(), badge_font.clone(), base);
                        let bw = g.size().x + 10.0;
                        let brect = Rect::from_center_size(
                            egui::pos2(x + bw / 2.0, cy),
                            Vec2::new(bw, row_h - 4.0),
                        );
                        ui.painter()
                            .rect_filled(brect, CornerRadius::same(t.radius.sm as u8), bg);
                        ui.painter()
                            .galley(egui::pos2(x + 5.0, cy - g.size().y / 2.0), g, base);
                        x += bw + 8.0;
                        // message — fg[1]
                        let g = util::galley(ui, &line.message, mono.clone(), t.fg[1]);
                        let clip = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
                        clip.galley(egui::pos2(x, cy - g.size().y / 2.0), g, t.fg[1]);
                    }
                });
            out
        });
        let out = inner.inner;

        // Un-pin when the user scrolls up; re-pin when they ride back down
        // to the tail.
        let hovered = ui.rect_contains_pointer(inner.response.rect);
        let raw_scroll = ui.input(|i| i.smooth_scroll_delta.y);
        let max_offset = (out.content_size.y - out.inner_rect.height()).max(0.0);
        let at_bottom = out.state.offset.y >= max_offset - 2.0;
        if hovered && raw_scroll > 0.0 && state.pinned && !at_bottom {
            state.pinned = false;
            outcome = outcome.merge(Outcome::Changed);
        } else if hovered && raw_scroll < 0.0 && !state.pinned && at_bottom {
            state.pinned = true;
            outcome = outcome.merge(Outcome::Changed);
        }

        ForgeResponse::new(inner.response, outcome)
    }
}

/// The Pinned/Follow toggle chip.
fn follow_chip(ui: &mut Ui, t: &Theme, pinned: bool) -> egui::Response {
    let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
    let label = if pinned {
        "● Following"
    } else {
        "○ Follow"
    };
    let g = util::galley(ui, label, font, t.fg[1]);
    let size = Vec2::new(g.size().x + 20.0, t.control.sm - 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, pinned, "Follow"));
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same((rect.height() / 2.0) as u8);
        let (fill, color) = if pinned {
            (t.accent.bg, t.accent.fg)
        } else if resp.hovered() {
            (t.bg[3], t.fg[0])
        } else {
            (t.bg[2], t.fg[1])
        };
        ui.painter().rect_filled(rect, radius, fill);
        let label = if pinned {
            "● Following"
        } else {
            "○ Follow"
        };
        let font = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        let g = util::galley(ui, label, font, color);
        ui.painter().galley(
            egui::pos2(
                rect.center().x - g.size().x / 2.0,
                rect.center().y - g.size().y / 2.0,
            ),
            g,
            color,
        );
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(msg: &str) -> LogLine {
        LogLine::new("12:00:00", Level::Info, msg)
    }

    #[test]
    fn filter_is_case_insensitive_substring_on_message() {
        assert!(line_matches(&line("Connection reset by peer"), ""));
        assert!(line_matches(&line("Connection reset by peer"), "reset"));
        assert!(line_matches(
            &line("Connection reset by peer"),
            "CONNECTION"
        ));
        assert!(!line_matches(&line("Connection reset by peer"), "timeout"));
        // Filter never matches on time.
        assert!(!line_matches(&line("hello"), "12:00"));
    }

    #[test]
    fn default_state_is_pinned_with_empty_filter() {
        let s = LogsState::default();
        assert!(s.pinned);
        assert!(s.filter.is_empty());
    }
}
