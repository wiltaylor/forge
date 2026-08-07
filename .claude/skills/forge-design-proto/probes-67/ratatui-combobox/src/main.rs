//! A settings screen with a region picker.
//!
//! Composition follows `reference/laws.md`: `app-shell` > `page-head` > content, and a
//! settings screen is `settings-layout` > `settings-section` > `settings-row`. No ratatui
//! implementation page exists for any of those five, so they are drawn here from the laws
//! rather than lifted from a page.

use std::io;

use probe_ratatui::{ComboBox, ComboBoxItem, ComboBoxState, Outcome, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Style, Stylize};
use ratatui::Terminal;

/// Every visible string is a parameter, so they live together here.
mod copy {
    pub const APP: &str = "Orbit";
    pub const NAV_HEADING: &str = "Account";
    pub const NAV_ITEMS: [&str; 4] = ["General", "Deployment", "Billing", "Access"];
    pub const EYEBROW: &str = "ACCOUNT";
    pub const TITLE: &str = "Settings";
    pub const SECTION: &str = "Deployment";
    pub const ROW_LABEL: &str = "Region";
    pub const ROW_HELP: &str = "Where new services deploy.";
    pub const ROW_UNSET: &str = "Nothing selected yet.";
    pub const PLACEHOLDER: &str = "Type to find a region";
    pub const NO_MATCH: &str = "No region matches. Clear the filter.";
    pub const UNAVAILABLE: &str = "unavailable";
    pub const KEYS: &str =
        "Type to filter · ↑↓ move · Enter selects · Esc closes · Tab refocuses · F2 theme · Ctrl+C quits";
}

/// The two regions this account cannot use.
const UNAVAILABLE: [&str; 2] = ["ap-east-1", "me-south-1"];

/// 40 regions. The city rides in the label, because the filter matches the label only.
const REGIONS: [(&str, &str); 40] = [
    ("us-east-1", "N. Virginia"),
    ("us-east-2", "Ohio"),
    ("us-central-1", "Dallas"),
    ("us-west-1", "N. California"),
    ("us-west-2", "Oregon"),
    ("us-gov-east-1", "Ashburn"),
    ("us-gov-west-1", "Boise"),
    ("ca-central-1", "Montreal"),
    ("ca-west-1", "Calgary"),
    ("sa-east-1", "Sao Paulo"),
    ("sa-west-1", "Santiago"),
    ("sa-north-1", "Bogota"),
    ("eu-west-1", "Ireland"),
    ("eu-west-2", "London"),
    ("eu-west-3", "Paris"),
    ("eu-central-1", "Frankfurt"),
    ("eu-central-2", "Zurich"),
    ("eu-north-1", "Stockholm"),
    ("eu-north-2", "Helsinki"),
    ("eu-south-1", "Milan"),
    ("eu-south-2", "Madrid"),
    ("eu-east-1", "Warsaw"),
    ("ap-south-1", "Mumbai"),
    ("ap-south-2", "Hyderabad"),
    ("ap-southeast-1", "Singapore"),
    ("ap-southeast-2", "Sydney"),
    ("ap-southeast-3", "Jakarta"),
    ("ap-southeast-4", "Melbourne"),
    ("ap-southeast-5", "Auckland"),
    ("ap-northeast-1", "Tokyo"),
    ("ap-northeast-2", "Seoul"),
    ("ap-northeast-3", "Osaka"),
    ("ap-northeast-4", "Sapporo"),
    ("ap-east-1", "Hong Kong"),
    ("ap-east-2", "Taipei"),
    ("me-south-1", "Bahrain"),
    ("me-central-1", "Dubai"),
    ("af-south-1", "Cape Town"),
    ("af-north-1", "Casablanca"),
    ("il-central-1", "Tel Aviv"),
];

fn regions() -> Vec<ComboBoxItem> {
    REGIONS
        .iter()
        .map(|(id, city)| {
            let item = ComboBoxItem::new(*id, format!("{id} · {city}"));
            if UNAVAILABLE.contains(id) {
                item.disabled(true).hint(copy::UNAVAILABLE)
            } else {
                item
            }
        })
        .collect()
}

struct App {
    theme: Theme,
    dark: bool,
    regions: Vec<ComboBoxItem>,
    picker: ComboBoxState,
    /// The committed value. The control page puts it in the caller's hands.
    region: Option<String>,
}

impl App {
    fn new() -> Self {
        let regions = regions();
        let mut picker = ComboBoxState::new();
        // Nothing is selected when the screen opens, and the popup starts closed so it
        // cannot cover the screen before the user asks for it.
        picker.refilter(&regions);
        Self {
            theme: Theme::dark(),
            dark: true,
            regions,
            picker,
            region: None,
        }
    }

    fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        self.theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|frame| draw(frame.area(), frame.buffer_mut(), &mut app))?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
                if key.code == KeyCode::F(2) {
                    app.toggle_theme();
                    continue;
                }
                // Tab moves between controls. There is one here, so it lands back on the
                // field — and focusing the field opens the popup and selects its text.
                if key.code == KeyCode::Tab {
                    app.picker.focus(&app.regions);
                    continue;
                }

                let outcome = app.picker.handle_key(key, &app.regions);
                if outcome == Outcome::Submitted {
                    app.region = app
                        .picker
                        .selected()
                        .map(|i| app.regions[i].value.clone());
                }
                // Ignored bubbles: Escape with the popup already closed leaves the screen.
                if !outcome.is_handled() && key.code == KeyCode::Esc {
                    return Ok(());
                }
            }
            Event::Mouse(mouse) => {
                if app.picker.handle_mouse(mouse, &app.regions) == Outcome::Submitted {
                    app.region = app
                        .picker
                        .selected()
                        .map(|i| app.regions[i].value.clone());
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// app-shell
// ---------------------------------------------------------------------------

const SIDEBAR: u16 = 20;
const LABEL_COLUMN: u16 = 18;

fn draw(area: Rect, buf: &mut Buffer, app: &mut App) {
    let theme = app.theme;
    buf.set_style(area, Style::default().bg(theme.bg[0]).fg(theme.fg[0]));

    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SIDEBAR), Constraint::Min(1)])
        .split(shell[0]);

    // The key bar goes down first: the popup is an overlay and must paint over it, never
    // the other way round.
    draw_keys(shell[1], buf, app);
    draw_nav(body[0], buf, app);

    // Every 1px division is one cell here.
    for y in body[1].top()..body[1].bottom() {
        buf.set_string(body[1].x, y, "│", Style::default().fg(theme.border.default));
    }
    let main = Rect {
        x: body[1].x + 2,
        y: body[1].y,
        width: body[1].width.saturating_sub(3),
        height: body[1].height,
    };
    draw_main(main, buf, app);
}

/// `nav-section` and `nav-link`.
fn draw_nav(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme;
    if area.width < 4 || area.height < 4 {
        return;
    }
    buf.set_string(
        area.x + 1,
        area.y,
        copy::APP,
        Style::default().fg(theme.fg[0]).bold(),
    );
    buf.set_string(
        area.x + 1,
        area.y + 2,
        copy::NAV_HEADING.to_uppercase(),
        Style::default().fg(theme.fg[2]),
    );
    for (i, item) in copy::NAV_ITEMS.iter().enumerate() {
        let y = area.y + 4 + i as u16;
        if y >= area.bottom() {
            break;
        }
        // The current screen. The accent is a marker and a text colour, never a fill.
        let current = i == 1;
        let style = if current {
            Style::default().fg(theme.accent.base)
        } else {
            Style::default().fg(theme.fg[1])
        };
        buf.set_string(area.x, y, if current { ">" } else { " " }, style);
        buf.set_string(area.x + 2, y, *item, style);
    }
}

/// `page-head` > `settings-layout`.
fn draw_main(area: Rect, buf: &mut Buffer, app: &mut App) {
    let theme = app.theme;
    if area.height < 8 || area.width < 24 {
        return;
    }
    // page-head: an eyebrow, the title, and a rule. No secondary actions live here.
    buf.set_string(
        area.x,
        area.y + 1,
        copy::EYEBROW,
        Style::default().fg(theme.fg[2]),
    );
    buf.set_string(
        area.x,
        area.y + 2,
        copy::TITLE,
        Style::default().fg(theme.fg[0]).bold(),
    );
    rule(
        Rect {
            x: area.x,
            y: area.y + 4,
            width: area.width,
            height: 1,
        },
        buf,
        theme.border.default,
    );

    // settings-layout: one column at every width.
    let layout = Rect {
        x: area.x,
        y: area.y + 6,
        width: area.width,
        height: area.height - 6,
    };
    draw_section(layout, buf, app);
}

/// `settings-section` — a heading, not a card.
fn draw_section(area: Rect, buf: &mut Buffer, app: &mut App) {
    let theme = app.theme;
    buf.set_string(
        area.x,
        area.y,
        copy::SECTION,
        Style::default().fg(theme.fg[0]).bold(),
    );
    rule(
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: 1,
        },
        buf,
        theme.border.subtle,
    );
    draw_region_row(
        Rect {
            x: area.x,
            y: area.y + 3,
            width: area.width,
            height: area.height.saturating_sub(3),
        },
        buf,
        app,
    );
}

/// `settings-row` — one control, the label on the left, the control on the right, and the
/// help text under the label.
fn draw_region_row(area: Rect, buf: &mut Buffer, app: &mut App) {
    let theme = app.theme;
    if area.width < LABEL_COLUMN + 24 {
        return;
    }
    buf.set_string(
        area.x,
        area.y,
        copy::ROW_LABEL,
        Style::default().fg(theme.fg[0]),
    );
    buf.set_string(
        area.x,
        area.y + 1,
        copy::ROW_HELP,
        Style::default().fg(theme.fg[2]),
    );
    // The committed value, in words. Truncated values stay reachable as a detail line.
    let detail = match &app.region {
        Some(region) => format!("Deploys to {region}."),
        None => copy::ROW_UNSET.to_string(),
    };
    buf.set_string(area.x, area.y + 2, detail, Style::default().fg(theme.fg[2]));

    let control = Rect {
        x: area.x + LABEL_COLUMN,
        y: area.y,
        width: (area.width - LABEL_COLUMN).min(48),
        height: 1,
    };
    // The widget will not flip the popup above the field, so the caller keeps it inside
    // the content area rather than letting it run under the key bar.
    let room = area.bottom().saturating_sub(control.y + 1);
    let combobox = ComboBox::new(&app.regions, &app.theme)
        .focused(true)
        .placeholder(copy::PLACEHOLDER)
        .empty_text(copy::NO_MATCH)
        .max_rows(room.min(10));
    ratatui::widgets::StatefulWidget::render(combobox, control, buf, &mut app.picker);
}

/// A key-hint bar. Forge names no such control, so this is plain help text inside the
/// shell, not a Forge name.
fn draw_keys(area: Rect, buf: &mut Buffer, app: &App) {
    let theme = app.theme;
    buf.set_style(area, Style::default().bg(theme.bg[1]));
    let text: String = copy::KEYS.chars().take(area.width as usize).collect();
    buf.set_string(
        area.x + 1,
        area.y,
        text,
        Style::default().bg(theme.bg[1]).fg(theme.fg[2]),
    );
}

fn rule(area: Rect, buf: &mut Buffer, colour: ratatui::style::Color) {
    let line = "─".repeat(area.width as usize);
    buf.set_string(area.x, area.y, line, Style::default().fg(colour));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn snapshot(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw(frame.area(), frame.buffer_mut(), app))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .chunks(width as usize)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn there_are_forty_regions_and_two_are_unavailable() {
        let items = regions();
        assert_eq!(items.len(), 40);
        assert_eq!(items.iter().filter(|i| i.disabled).count(), 2);
    }

    /// Prints the screen in both themes. `cargo test -- --nocapture` to look at it.
    #[test]
    fn the_screen_draws_in_both_themes() {
        let mut app = App::new();
        println!("--- dark, closed ---\n{}", snapshot(&mut app, 100, 24));

        app.picker
            .handle_key(key(KeyCode::Down), &app.regions.clone());
        println!("--- dark, open ---\n{}", snapshot(&mut app, 100, 24));

        for c in "syd".chars() {
            let items = app.regions.clone();
            app.picker.handle_key(key(KeyCode::Char(c)), &items);
        }
        println!("--- dark, filtered ---\n{}", snapshot(&mut app, 100, 24));

        for c in "zzz".chars() {
            let items = app.regions.clone();
            app.picker.handle_key(key(KeyCode::Char(c)), &items);
        }
        println!("--- dark, no match ---\n{}", snapshot(&mut app, 100, 24));

        app.toggle_theme();
        println!("--- light, no match ---\n{}", snapshot(&mut app, 100, 24));
    }

    /// The two unavailable regions carry a word, never colour alone, and cannot commit.
    #[test]
    fn the_unavailable_regions_show_and_refuse() {
        let mut app = App::new();
        let items = app.regions.clone();
        for c in "ap-east".chars() {
            app.picker.handle_key(key(KeyCode::Char(c)), &items);
        }
        println!("--- unavailable ---\n{}", snapshot(&mut app, 100, 16));
        assert_eq!(
            app.picker.handle_key(key(KeyCode::Enter), &items),
            Outcome::Consumed
        );
        assert_eq!(app.picker.selected(), None);

        // The next one down is available and commits, and the marker follows it.
        app.picker.handle_key(key(KeyCode::Down), &items);
        assert_eq!(
            app.picker.handle_key(key(KeyCode::Enter), &items),
            Outcome::Submitted
        );
        app.region = app.picker.selected().map(|i| app.regions[i].value.clone());
        assert_eq!(app.region.as_deref(), Some("ap-east-2"));
        app.picker.focus(&items);
        println!("--- committed ---\n{}", snapshot(&mut app, 100, 16));
    }

    fn key(code: KeyCode) -> ratatui::crossterm::event::KeyEvent {
        ratatui::crossterm::event::KeyEvent::new(code, KeyModifiers::NONE)
    }
}
