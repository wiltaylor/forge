//! A deploy screen with a row of three actions.
//!
//! `laws.md` composes a screen as app-shell > page-head > content, but the
//! ratatui implementation pages for `app-shell` and `page-head` are "Not
//! written", so the title block below is drawn inline rather than through an
//! invented control type. The three actions sit in the content, not in the
//! head — the head takes one primary action only.

mod forge;

use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use ratatui::{Frame, Terminal};

use forge::{Button, Outcome, Theme, Variant};

/// The glyph in the leading slot of Deploy. It is reserved whether or not a
/// deploy is running, so the spinner does not resize the button.
const DEPLOY_GLYPH: char = '↑';

/// How often the spinner advances one frame.
const TICK: Duration = Duration::from_millis(100);

/// Which action holds focus. Exactly one thing does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Deploy,
    DryRun,
    Cancel,
}

const ORDER: [Action; 3] = [Action::Deploy, Action::DryRun, Action::Cancel];

struct App {
    theme: Theme,
    /// Destructive actions are never the default focus target, so this starts on
    /// Deploy.
    focus: usize,
    in_flight: bool,
    tick: u64,
    status: String,
    running: bool,
}

impl App {
    fn new() -> Self {
        Self {
            theme: Theme::dark(),
            focus: 0,
            in_flight: false,
            tick: 0,
            status: "Idle".to_string(),
            running: true,
        }
    }

    fn focused(&self) -> Action {
        ORDER[self.focus]
    }

    /// Build one action's button. The row is rebuilt every frame; the app owns
    /// the state.
    fn button(&self, action: Action) -> Button<'_> {
        let focused = self.focused() == action;
        match action {
            Action::Deploy => Button::new("Deploy")
                .variant(Variant::Primary)
                .icon(DEPLOY_GLYPH)
                .loading(self.in_flight)
                .tick(self.tick)
                .focused(focused)
                .theme(&self.theme),
            Action::DryRun => Button::new("Dry run")
                .variant(Variant::Default)
                .disabled(self.in_flight)
                .focused(focused)
                .theme(&self.theme),
            Action::Cancel => Button::new("Cancel deployment")
                .variant(Variant::Danger)
                .focused(focused)
                .theme(&self.theme),
        }
    }

    /// Tab moves between controls, and steps over anything that cannot be
    /// activated.
    fn move_focus(&mut self, step: isize) {
        let len = ORDER.len() as isize;
        for hop in 1..=len {
            let next = (self.focus as isize + step * hop).rem_euclid(len) as usize;
            if self.button(ORDER[next]).is_interactive() {
                self.focus = next;
                return;
            }
        }
    }

    fn submit(&mut self, action: Action) {
        match action {
            Action::Deploy => {
                self.in_flight = true;
                self.status = "Deploying — 3 of 8 services".to_string();
                // Dry run just became disabled; do not leave focus on it.
                if !self.button(self.focused()).is_interactive() {
                    self.move_focus(1);
                }
            }
            Action::DryRun => {
                self.status = "Dry run finished — no changes".to_string();
            }
            Action::Cancel => {
                self.in_flight = false;
                self.status = "Deployment cancelled".to_string();
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Offer the key to the focused control first. Anything but `Ignored`
        // stops the routing.
        let action = self.focused();
        let outcome = self.button(action).handle_key(key);
        if outcome == Outcome::Submitted {
            self.submit(action);
            return;
        }
        if outcome.is_handled() {
            return;
        }

        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Tab => self.move_focus(1),
            KeyCode::BackTab => self.move_focus(-1),
            KeyCode::Char('t') => {
                self.in_flight = !self.in_flight;
                self.status = if self.in_flight {
                    "Deploying — 3 of 8 services".to_string()
                } else {
                    "Idle".to_string()
                };
                if !self.button(self.focused()).is_interactive() {
                    self.move_focus(1);
                }
            }
            KeyCode::Char('l') => {
                self.theme = if self.theme == Theme::dark() {
                    Theme::light()
                } else {
                    Theme::dark()
                };
            }
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            _ => {}
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // The page surface.
        let surface = Style::new().fg(self.theme.fg[0]).bg(self.theme.bg[0]);
        let blank = " ".repeat(area.width as usize);
        for y in area.top()..area.bottom() {
            buf.set_stringn(area.x, y, &blank, area.width as usize, surface);
        }

        let x = area.x + 2;
        let room = area.width.saturating_sub(4) as usize;

        // Title and its dim sub-line. Sentence case.
        let title = Style::new().fg(self.theme.fg[0]).bg(self.theme.bg[0]);
        let dim = Style::new().fg(self.theme.fg[2]).bg(self.theme.bg[0]);
        buf.set_stringn(x, area.y + 1, "Deploy", room, title);
        buf.set_stringn(x, area.y + 2, "web-api · production", room, dim);

        // Status. A colour never carries the meaning alone, so it is paired
        // with the word.
        let status_style = if self.in_flight {
            Style::new().fg(self.theme.warning.base).bg(self.theme.bg[0])
        } else {
            dim
        };
        buf.set_stringn(x, area.y + 4, self.status.as_str(), room, status_style);

        // The action row. Buttons do not stretch, so lay them out by width.
        let mut cursor = x;
        let row = area.y + 6;
        for action in ORDER {
            let button = self.button(action);
            let width = button.width();
            if cursor + width > area.right() {
                break;
            }
            button.render(Rect::new(cursor, row, width, 1), buf);
            cursor += width;
        }

        let hint = "Tab move · Enter or Space activate · t toggle in-flight · l theme · q quit";
        buf.set_stringn(x, area.y + 8, hint, room, dim);
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut app = App::new();
    let mut last = Instant::now();

    while app.running {
        terminal.draw(|frame| app.draw(frame))?;

        let timeout = TICK.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        // The caller drives the spinner tick; the control owns no timer.
        if last.elapsed() >= TICK {
            last = Instant::now();
            if app.in_flight {
                app.tick = app.tick.wrapping_add(1);
            }
        }
    }

    Ok(())
}
