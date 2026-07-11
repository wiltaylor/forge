//! Month-grid calendar. Monday-start 6×7 grid (web parity: always 42 cells),
//! ISO `YYYY-MM-DD` value strings, min/max clamping, ‹ › month nav.

use crate::response::{ForgeResponse, Outcome};
use crate::theme::{FontWeight, Theme};
use crate::widgets::primitives::Glyph;
use egui::{
    CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType,
};
use time::{Date, Duration, Month, OffsetDateTime};

const CELL_W: f32 = 32.0;
const CELL_H: f32 = 28.0;
const GAP: f32 = 2.0;
const GRID_W: f32 = 7.0 * CELL_W + 6.0 * GAP;
const HEAD_H: f32 = 28.0;
const DOW_H: f32 = 18.0;
const DOW: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Viewed month + selected ISO date. Plain app-owned data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarState {
    /// `(year, month 1..=12)` currently in view.
    pub month: (i32, u8),
    /// Selected date as ISO `YYYY-MM-DD`.
    pub value: Option<String>,
}

impl CalendarState {
    /// View the current month with nothing selected.
    pub fn today() -> CalendarState {
        let d = today();
        CalendarState {
            month: (d.year(), u8::from(d.month())),
            value: None,
        }
    }
}

impl Default for CalendarState {
    fn default() -> CalendarState {
        CalendarState::today()
    }
}

fn today() -> Date {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date()
}

pub(crate) fn parse_iso(s: &str) -> Option<Date> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u8 = parts.next()?.parse().ok()?;
    let d: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Date::from_calendar_date(y, Month::try_from(m).ok()?, d).ok()
}

pub(crate) fn format_iso(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}

/// Days in a month, leap years included. (Widget layout never needs it —
/// the 42-cell grid is date-arithmetic driven — but the math stays covered.)
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn days_in_month(year: i32, month: u8) -> u8 {
    Month::try_from(month).map_or(0, |m| m.length(year))
}

/// Monday-start weekday offset of the 1st of the month (0 = Monday).
pub(crate) fn first_offset(year: i32, month: u8) -> u8 {
    Month::try_from(month)
        .ok()
        .and_then(|m| Date::from_calendar_date(year, m, 1).ok())
        .map_or(0, |d| d.weekday().number_days_from_monday())
}

/// Shift a `(year, month)` view by `delta` months.
pub(crate) fn add_months(month: (i32, u8), delta: i32) -> (i32, u8) {
    let idx = month.0 * 12 + i32::from(month.1) - 1 + delta;
    (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u8)
}

/// The date shown in grid cell `idx` (0..42) of the viewed month.
fn cell_date(year: i32, month: u8, idx: u8) -> Option<Date> {
    let m = Month::try_from(month).ok()?;
    let first = Date::from_calendar_date(year, m, 1).ok()?;
    let lead = i64::from(first_offset(year, month));
    first.checked_add(Duration::days(i64::from(idx) - lead))
}

/// Inline month calendar bound to a [`CalendarState`]. Emits
/// [`Outcome::Changed`] when a day is picked.
pub struct Calendar<'a> {
    state: &'a mut CalendarState,
    min: Option<&'a str>,
    max: Option<&'a str>,
}

impl<'a> Calendar<'a> {
    pub fn new(state: &'a mut CalendarState) -> Calendar<'a> {
        Calendar {
            state,
            min: None,
            max: None,
        }
    }

    /// Earliest selectable ISO date (inclusive).
    pub fn min(mut self, min: &'a str) -> Self {
        self.min = Some(min);
        self
    }

    /// Latest selectable ISO date (inclusive).
    pub fn max(mut self, max: &'a str) -> Self {
        self.max = Some(max);
        self
    }

    pub fn show(self, ui: &mut Ui) -> ForgeResponse {
        let t = Theme::of(ui.ctx());
        let height = HEAD_H + 6.0 + DOW_H + 4.0 + 6.0 * CELL_H + 5.0 * GAP;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(GRID_W, height), Sense::hover());
        let mut outcome = Outcome::Ignored;
        let mut union: Response = response.clone();
        let visible = ui.is_rect_visible(rect);

        let (year, month) = self.state.month;
        let min_date = self.min.and_then(parse_iso);
        let max_date = self.max.and_then(parse_iso);
        let selected = self.state.value.as_deref().and_then(parse_iso);
        let now = today();

        // Header: ‹ nav, "Month YYYY", › nav.
        let head = Rect::from_min_size(rect.min, Vec2::new(GRID_W, HEAD_H));
        for (side, glyph, label, delta) in [
            (0, Glyph::ChevronLeft, "Previous month", -1),
            (1, Glyph::ChevronRight, "Next month", 1),
        ] {
            let x = if side == 0 {
                head.min.x
            } else {
                head.max.x - HEAD_H
            };
            let btn = Rect::from_min_size(Pos2::new(x, head.min.y), Vec2::splat(HEAD_H));
            let resp = ui.interact(btn, response.id.with(("nav", side)), Sense::click());
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
            if visible {
                if resp.hovered() {
                    ui.painter()
                        .rect_filled(btn, CornerRadius::same(t.radius.md as u8), t.bg[2]);
                }
                let g = ui.painter().layout_no_wrap(
                    glyph.as_str().to_owned(),
                    t.font(ui.ctx(), FontWeight::Regular, t.type_scale.base),
                    t.fg[1],
                );
                ui.painter()
                    .galley(btn.center() - g.size() / 2.0, g, t.fg[1]);
            }
            if resp.clicked() {
                self.state.month = add_months(self.state.month, delta);
                outcome = outcome.merge(Outcome::Consumed);
            }
            union = union.union(resp);
        }
        if visible {
            let title = format!("{} {}", MONTHS[(month.max(1) - 1).min(11) as usize], year);
            let g = ui.painter().layout_no_wrap(
                title,
                t.font(ui.ctx(), FontWeight::Medium, t.type_scale.base),
                t.fg[0],
            );
            ui.painter()
                .galley(head.center() - g.size() / 2.0, g, t.fg[0]);

            // Weekday header.
            let dow_font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.xs);
            for (c, dow) in DOW.iter().enumerate() {
                let cx = rect.min.x + c as f32 * (CELL_W + GAP) + CELL_W / 2.0;
                let g = ui
                    .painter()
                    .layout_no_wrap((*dow).to_owned(), dow_font.clone(), t.fg[2]);
                ui.painter().galley(
                    Pos2::new(
                        cx - g.size().x / 2.0,
                        head.max.y + 6.0 + (DOW_H - g.size().y) / 2.0,
                    ),
                    g,
                    t.fg[2],
                );
            }
        }

        // 6×7 day grid (always 42 cells, web parity).
        let (year, month) = self.state.month; // re-read: nav may have run
        let grid_top = rect.min.y + HEAD_H + 6.0 + DOW_H + 4.0;
        let radius = CornerRadius::same(t.radius.sm as u8);
        let day_font = t.font(ui.ctx(), FontWeight::Regular, t.type_scale.sm);
        let day_font_med = t.font(ui.ctx(), FontWeight::Medium, t.type_scale.sm);
        for idx in 0..42u8 {
            let Some(date) = cell_date(year, month, idx) else {
                continue;
            };
            let (row, col) = (f32::from(idx / 7), f32::from(idx % 7));
            let cell = Rect::from_min_size(
                Pos2::new(
                    rect.min.x + col * (CELL_W + GAP),
                    grid_top + row * (CELL_H + GAP),
                ),
                Vec2::new(CELL_W, CELL_H),
            );
            let iso = format_iso(date);
            let disabled = min_date.is_some_and(|m| date < m) || max_date.is_some_and(|m| date > m);
            let out = u8::from(date.month()) != month || date.year() != year;
            let is_selected = selected == Some(date);
            let is_today = date == now;

            let sense = if disabled {
                Sense::hover()
            } else {
                Sense::click()
            };
            let resp = ui.interact(cell, response.id.with(("day", idx)), sense);
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, !disabled, &iso));

            if visible {
                if is_selected {
                    ui.painter().rect_filled(cell, radius, t.accent.base);
                } else if resp.hovered() && !disabled {
                    ui.painter().rect_filled(cell, radius, t.bg[2]);
                }
                if is_today && !is_selected {
                    ui.painter().rect_stroke(
                        cell,
                        radius,
                        Stroke::new(1.0, t.accent.base),
                        StrokeKind::Inside,
                    );
                }
                let color = if is_selected {
                    t.accent.contrast
                } else if disabled || out {
                    t.fg[3]
                } else {
                    t.fg[1]
                };
                let font = if is_selected || is_today {
                    day_font_med.clone()
                } else {
                    day_font.clone()
                };
                let g = ui
                    .painter()
                    .layout_no_wrap(date.day().to_string(), font, color);
                ui.painter()
                    .galley(cell.center() - g.size() / 2.0, g, color);
            }

            if resp.clicked() && !disabled {
                self.state.value = Some(iso);
                // Selecting an adjacent-month day pulls that month into view.
                self.state.month = (date.year(), u8::from(date.month()));
                outcome = outcome.merge(Outcome::Changed);
            }
            union = union.union(resp);
        }

        ForgeResponse::new(union, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_round_trip() {
        for iso in ["2026-07-11", "2024-02-29", "1999-12-31", "2020-01-01"] {
            let d = parse_iso(iso).expect(iso);
            assert_eq!(format_iso(d), iso);
        }
        assert!(parse_iso("2023-02-29").is_none()); // not a leap year
        assert!(parse_iso("2023-13-01").is_none());
        assert!(parse_iso("nonsense").is_none());
        assert!(parse_iso("2023-01-01-01").is_none());
    }

    #[test]
    fn monday_start_offsets() {
        assert_eq!(first_offset(2026, 7), 2); // 1 Jul 2026 = Wednesday
        assert_eq!(first_offset(2026, 6), 0); // 1 Jun 2026 = Monday
        assert_eq!(first_offset(2026, 2), 6); // 1 Feb 2026 = Sunday
        assert_eq!(first_offset(2024, 1), 0); // 1 Jan 2024 = Monday
    }

    #[test]
    fn leap_year_february() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29); // century leap
        assert_eq!(days_in_month(1900, 2), 28); // century non-leap
        assert_eq!(days_in_month(2026, 7), 31);
        assert_eq!(days_in_month(2026, 4), 30);
    }

    #[test]
    fn month_navigation_wraps_years() {
        assert_eq!(add_months((2026, 12), 1), (2027, 1));
        assert_eq!(add_months((2026, 1), -1), (2025, 12));
        assert_eq!(add_months((2026, 7), 18), (2028, 1));
        assert_eq!(add_months((2026, 7), -7), (2025, 12));
    }

    #[test]
    fn grid_cells_cover_adjacent_months() {
        // July 2026 leads with Mon 29 + Tue 30 June, trails into August.
        assert_eq!(cell_date(2026, 7, 0), parse_iso("2026-06-29"));
        assert_eq!(cell_date(2026, 7, 2), parse_iso("2026-07-01"));
        assert_eq!(cell_date(2026, 7, 41), parse_iso("2026-08-09"));
    }

    #[test]
    fn min_max_clamping_semantics() {
        let min = parse_iso("2026-07-05").unwrap();
        let max = parse_iso("2026-07-20").unwrap();
        let inside = parse_iso("2026-07-11").unwrap();
        let below = parse_iso("2026-07-04").unwrap();
        let above = parse_iso("2026-07-21").unwrap();
        assert!(!(inside < min || inside > max));
        assert!(below < min);
        assert!(above > max);
        // Bounds themselves stay selectable (inclusive).
        assert!(!(min < min || min > max));
        assert!(!(max < min || max > max));
    }
}
