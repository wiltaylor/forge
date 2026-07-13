//! Calendar + DatePicker (cargo feature `calendar`, uses the `time` crate).

use crate::event::{clicked, in_area, is_press, left_down, scroll_delta, Outcome};
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Clear, StatefulWidget, Widget};
use time::{Date, Duration, Month, OffsetDateTime, Weekday};

fn today() -> Date {
    OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .date()
}

fn month_name(m: Month) -> &'static str {
    match m {
        Month::January => "January",
        Month::February => "February",
        Month::March => "March",
        Month::April => "April",
        Month::May => "May",
        Month::June => "June",
        Month::July => "July",
        Month::August => "August",
        Month::September => "September",
        Month::October => "October",
        Month::November => "November",
        Month::December => "December",
    }
}

/// Selected date + viewed month. Arrows move by day/week, PgUp/PgDn by
/// month, `t` jumps to today, Enter submits.
#[derive(Clone, Copy, Debug)]
pub struct CalendarState {
    pub selected: Date,
    view: (i32, Month),
    area: Rect,
}

impl Default for CalendarState {
    fn default() -> CalendarState {
        CalendarState::new(today())
    }
}

impl CalendarState {
    pub fn new(selected: Date) -> CalendarState {
        CalendarState {
            selected,
            view: (selected.year(), selected.month()),
            area: Rect::default(),
        }
    }

    /// Click a day to select it (clicking the selection submits); the wheel
    /// pages months.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta != 0 && in_area(ev, self.area) {
            return self.shift_month(delta > 0);
        }
        if !left_down(ev) || !in_area(ev, self.area) || ev.row < self.area.y + 2 {
            return Outcome::Ignored;
        }
        let (year, month) = self.view;
        let Ok(first) = Date::from_calendar_date(year, month, 1) else {
            return Outcome::Consumed;
        };
        let lead = first.weekday().number_days_from_monday() as u16;
        let col = (ev.column - self.area.x) / 3;
        if col > 6 {
            return Outcome::Consumed;
        }
        let row = ev.row - self.area.y - 2;
        let idx = row * 7 + col;
        if idx < lead {
            return Outcome::Consumed;
        }
        let day = (idx - lead + 1) as u8;
        if day > month.length(year) {
            return Outcome::Consumed;
        }
        let Ok(date) = Date::from_calendar_date(year, month, day) else {
            return Outcome::Consumed;
        };
        if date == self.selected {
            Outcome::Submitted
        } else {
            self.select(date)
        }
    }

    pub fn view(&self) -> (i32, Month) {
        self.view
    }

    fn select(&mut self, date: Date) -> Outcome {
        self.selected = date;
        self.view = (date.year(), date.month());
        Outcome::Changed
    }

    fn shift_month(&mut self, forward: bool) -> Outcome {
        let (y, m) = self.view;
        let (ny, nm) = if forward {
            match m {
                Month::December => (y + 1, Month::January),
                m => (y, m.next()),
            }
        } else {
            match m {
                Month::January => (y - 1, Month::December),
                m => (y, m.previous()),
            }
        };
        let day = self.selected.day().min(nm.length(ny));
        if let Ok(date) = Date::from_calendar_date(ny, nm, day) {
            self.select(date)
        } else {
            Outcome::Consumed
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Left => self.select(self.selected - Duration::days(1)),
            KeyCode::Right => self.select(self.selected + Duration::days(1)),
            KeyCode::Up => self.select(self.selected - Duration::days(7)),
            KeyCode::Down => self.select(self.selected + Duration::days(7)),
            KeyCode::PageUp => self.shift_month(false),
            KeyCode::PageDown => self.shift_month(true),
            KeyCode::Char('t') => self.select(today()),
            KeyCode::Enter => Outcome::Submitted,
            KeyCode::Esc => Outcome::Cancelled,
            _ => Outcome::Ignored,
        }
    }
}

/// Month grid (Monday-first): dim adjacent-month blanks, today in accent
/// text, the selection on an accent chip. Wants 10 rows × 22 columns.
#[derive(Clone, Debug, Default)]
pub struct Calendar<'a> {
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Calendar<'a> {
    pub fn new() -> Calendar<'a> {
        Calendar::default()
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub const WIDTH: u16 = 22;
    pub const HEIGHT: u16 = 9;
}

impl<'a> StatefulWidget for Calendar<'a> {
    type State = CalendarState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut CalendarState) {
        state.area = Rect::new(
            area.x,
            area.y,
            area.width.min(21),
            area.height.min(Calendar::HEIGHT),
        );
        if area.width < 21 || area.height < 3 {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let (year, month) = state.view;
        // Header.
        let header = format!("{} {}", month_name(month), year);
        let hx = area.x + (21u16.saturating_sub(header.len() as u16)) / 2;
        buf.set_string(
            hx,
            area.y,
            &header,
            Style::new().fg(t.fg[0]).add_modifier(if self.focused {
                Modifier::BOLD | Modifier::UNDERLINED
            } else {
                Modifier::BOLD
            }),
        );
        // Weekday row.
        buf.set_string(
            area.x,
            area.y + 1,
            "Mo Tu We Th Fr Sa Su",
            Style::new().fg(t.fg[3]),
        );
        // Day grid.
        let first = match Date::from_calendar_date(year, month, 1) {
            Ok(d) => d,
            Err(_) => return,
        };
        let lead = first.weekday().number_days_from_monday() as u16; // 0 = Monday
        let days = month.length(year);
        let now = today();
        for day in 1..=days {
            let idx = lead + day as u16 - 1;
            let row = idx / 7;
            let col = idx % 7;
            let y = area.y + 2 + row;
            if y >= area.y + area.height {
                break;
            }
            let x = area.x + col * 3;
            let date = Date::from_calendar_date(year, month, day).unwrap();
            let is_selected = date == state.selected;
            let is_today = date == now;
            let label = format!("{day:>2}");
            let style = if is_selected {
                Style::new()
                    .fg(t.accent.contrast)
                    .bg(t.accent.base)
                    .add_modifier(Modifier::BOLD)
            } else if is_today {
                Style::new().fg(t.accent.fg).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(t.fg[1])
            };
            buf.set_string(x, y, label, style);
        }
    }
}

/// Input-style field + Calendar popup (same overdraw pattern as `Select`).
#[derive(Clone, Debug, Default)]
pub struct DatePickerState {
    pub open: bool,
    pub cal: CalendarState,
    field: Rect,
}

impl DatePickerState {
    pub fn new(selected: Date) -> DatePickerState {
        DatePickerState {
            open: false,
            cal: CalendarState::new(selected),
            field: Rect::default(),
        }
    }

    pub fn selected(&self) -> Date {
        self.cal.selected
    }

    /// Click the field to open/close; click a day to pick it; click away to
    /// dismiss.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        if !self.open {
            if clicked(ev, self.field) {
                self.open = true;
                return Outcome::Consumed;
            }
            return Outcome::Ignored;
        }
        match self.cal.handle_mouse(ev) {
            Outcome::Submitted => {
                self.open = false;
                Outcome::Changed
            }
            Outcome::Ignored if left_down(ev) => {
                self.open = false;
                Outcome::Consumed
            }
            Outcome::Ignored => Outcome::Ignored,
            o => o,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        if !self.open {
            return match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Down => {
                    self.open = true;
                    Outcome::Consumed
                }
                _ => Outcome::Ignored,
            };
        }
        match self.cal.handle_key(key) {
            Outcome::Submitted => {
                self.open = false;
                Outcome::Changed
            }
            Outcome::Cancelled => {
                self.open = false;
                Outcome::Consumed
            }
            Outcome::Ignored => Outcome::Consumed, // trap while open
            o => o,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DatePicker<'a> {
    focused: bool,
    disabled: bool,
    theme: Option<&'a Theme>,
}

impl<'a> DatePicker<'a> {
    pub fn new() -> DatePicker<'a> {
        DatePicker::default()
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for DatePicker<'a> {
    type State = DatePickerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut DatePickerState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let edge = if self.focused && !self.disabled {
            t.accent.base
        } else {
            t.border.default
        };
        state.field = Rect::new(area.x, area.y, area.width, 1);
        buf.set_style(
            Rect::new(area.x, area.y, area.width, 1),
            Style::new().bg(t.bg[2]),
        );
        buf.set_string(area.x, area.y, "▎", Style::new().fg(edge).bg(t.bg[2]));
        let d = state.cal.selected;
        let label = format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day());
        buf.set_string(
            area.x + 1,
            area.y,
            &label,
            Style::new()
                .fg(if self.disabled { t.fg[3] } else { t.fg[0] })
                .bg(t.bg[2]),
        );
        if area.width >= 3 {
            buf.set_string(
                area.x + area.width - 2,
                area.y,
                if state.open { "▴" } else { "▾" },
                Style::new().fg(t.fg[2]).bg(t.bg[2]),
            );
        }
        if state.open && !self.disabled {
            let popup = Rect::new(
                area.x,
                area.y + 1,
                (Calendar::WIDTH + 2).max(area.width.min(Calendar::WIDTH + 2)),
                Calendar::HEIGHT + 2,
            )
            .intersection(buf.area);
            if popup.height >= 5 {
                Clear.render(popup, buf);
                let block = Block::bordered()
                    .border_style(Style::new().fg(t.border.strong).bg(t.bg[4]))
                    .style(Style::new().bg(t.bg[4]));
                let inner = block.inner(popup);
                block.render(popup, buf);
                Calendar::new()
                    .focused(self.focused)
                    .theme(t)
                    .render(inner, buf, &mut state.cal);
            }
        }
    }
}

const _: () = {
    // Compile-time reminder: keep Monday-first ordering consistent with the
    // weekday header above.
    let _ = Weekday::Monday;
};
