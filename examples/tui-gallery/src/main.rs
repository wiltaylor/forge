//! forge-tui gallery — the living catalogue of every widget, mirroring
//! `apps/gallery` on the web. Run with `just tui-gallery`.

mod sections;

use forge_tui::prelude::*;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

const NAV: FocusId = FocusId::new("nav");

pub struct Gallery {
    pub section: usize,
    pub mode: ColorMode,
    pub forms: sections::forms::FormsState,
    pub feedback: sections::feedback::FeedbackState,
}

pub const SECTIONS: &[&str] = &["Primitives", "Forms", "Feedback"];

impl Gallery {
    fn new() -> Gallery {
        Gallery {
            section: 0,
            mode: ColorMode::detect(),
            forms: Default::default(),
            feedback: Default::default(),
        }
    }
}

impl App for Gallery {
    fn draw(&mut self, frame: &mut Frame, ctx: &mut Ctx) {
        let t = ctx.theme.clone();
        let area = frame.area();
        frame.buffer_mut().set_style(area, Style::new().bg(t.bg[0]).fg(t.fg[0]));

        let [nav_area, content, status] = {
            let [main, status] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
            let [nav, content] =
                Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]).areas(main);
            [nav, content, status]
        };

        self.draw_nav(frame, nav_area, ctx, &t);
        self.draw_status(frame, status, &t);

        let content = content.inner(ratatui::layout::Margin::new(2, 1));
        match self.section {
            0 => sections::primitives::draw(frame, content, ctx, &t),
            1 => sections::forms::draw(frame, content, ctx, &t, &mut self.forms),
            _ => sections::feedback::draw(frame, content, ctx, &t, &mut self.feedback),
        }
    }

    fn on_event(&mut self, event: Event, ctx: &mut Ctx) {
        match event {
            Event::Key(key) => {
                let focused = ctx.focus.current();
                let outcome = if focused == Some(NAV) {
                    self.handle_nav_key(key)
                } else if self.section == 1 {
                    self.forms.handle_key(focused, key, ctx)
                } else if self.section == 2 {
                    self.feedback.handle_key(focused, key, ctx)
                } else {
                    Outcome::Ignored
                };
                if outcome.is_handled() {
                    return;
                }
                match key.code {
                    KeyCode::Char('q') => ctx.quit(),
                    KeyCode::Char('t') => {
                        let next = match ctx.theme.scheme {
                            Scheme::Dark => Theme::light(),
                            Scheme::Light => Theme::dark(),
                        };
                        ctx.theme = next.quantized(self.mode);
                    }
                    _ => {}
                }
            }
            Event::Paste(text) => {
                self.forms.paste(ctx.focus.current(), &text);
            }
            _ => {}
        }
    }
}

impl Gallery {
    fn handle_nav_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.section = self.section.saturating_sub(1);
                Outcome::Changed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.section = (self.section + 1).min(SECTIONS.len() - 1);
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }

    fn draw_nav(&mut self, frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme) {
        let buf = frame.buffer_mut();
        buf.set_style(area, Style::new().bg(t.bg[1]));
        let nav_focused = ctx.focus.register(NAV);
        if area.is_empty() {
            return;
        }
        let bottom = area.y + area.height;
        if area.y + 1 < bottom {
            buf.set_string(
                area.x + 2,
                area.y + 1,
                "◆ FORGE",
                Style::new().fg(t.accent.base).add_modifier(Modifier::BOLD),
            );
        }
        if area.y + 2 < bottom {
            buf.set_string(area.x + 2, area.y + 2, "tui gallery", Style::new().fg(t.fg[2]));
        }
        for (i, name) in SECTIONS.iter().enumerate() {
            let y = area.y + 4 + i as u16;
            if y >= bottom {
                break;
            }
            let active = i == self.section;
            if active {
                buf.set_string(area.x, y, "▎", Style::new().fg(t.accent.base).bg(t.bg[1]));
                buf.set_style(Rect::new(area.x, y, area.width, 1), Style::new().bg(t.bg[3]));
            }
            let mut style = Style::new()
                .fg(if active { t.fg[0] } else { t.fg[1] })
                .bg(if active { t.bg[3] } else { t.bg[1] });
            if active && nav_focused {
                style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            buf.set_string(area.x + 2, y, *name, style);
        }
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect, t: &Theme) {
        let buf = frame.buffer_mut();
        buf.set_style(area, Style::new().bg(t.bg[1]));
        if area.is_empty() {
            return;
        }
        buf.set_string(
            area.x + 1,
            area.y,
            "Tab focus · ↑/↓ section · t theme · q quit",
            Style::new().fg(t.fg[2]).bg(t.bg[1]),
        );
    }
}

fn main() -> forge_tui::Result<()> {
    let mut app = Gallery::new();
    forge_tui::runtime::run(&mut app, Theme::dark(), RunOptions::default())
}

#[cfg(test)]
mod smoke {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_all(width: u16, height: u16) {
        for section in 0..SECTIONS.len() {
            let mut app = Gallery::new();
            app.section = section;
            let mut ctx = forge_tui::runtime::test_ctx(Theme::dark());
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| {
                    ctx.focus.begin_frame();
                    app.draw(frame, &mut ctx);
                })
                .unwrap();
        }
    }

    #[test]
    fn sections_render_at_standard_size() {
        render_all(80, 24);
    }

    #[test]
    fn sections_survive_degenerate_sizes() {
        render_all(20, 5);
        render_all(5, 2);
        render_all(1, 1);
    }

    #[test]
    #[ignore = "visual dump: cargo test -p tui-gallery -- --ignored --nocapture"]
    fn dump_sections() {
        for section in 0..SECTIONS.len() {
            let mut app = Gallery::new();
            app.section = section;
            let mut ctx = forge_tui::runtime::test_ctx(Theme::dark());
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
            terminal
                .draw(|frame| {
                    ctx.focus.begin_frame();
                    app.draw(frame, &mut ctx);
                })
                .unwrap();
            println!("═══ {} ═══", SECTIONS[section]);
            let buf = terminal.backend().buffer();
            for y in 0..30u16 {
                let line: String = (0..100u16).map(|x| buf[(x, y)].symbol()).collect();
                println!("{}", line.trim_end());
            }
        }
    }
}
