//! forge-tui gallery — the living catalogue of every widget, mirroring
//! `apps/gallery` on the web. Run with `just tui-gallery`.

mod sections;

use forge_tui::prelude::*;
use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::Frame;

const NAV: FocusId = FocusId::new("nav");

pub const SECTIONS: &[&str] = &[
    "Primitives",
    "Feedback",
    "Forms",
    "Pickers",
    "Structure",
    "Overlays",
    "Data",
    "Files",
    "Board",
    "Charts",
    "Date",
    "Markdown",
    "Chat",
    "Code",
    "Terminal",
    "Flow",
    "Effects",
];

pub struct Gallery {
    pub mode: ColorMode,
    pub shell: ShellState,
    pub forms: sections::forms::FormsState,
    pub pickers: sections::pickers::PickersState,
    pub structure: sections::structure::StructureState,
    pub feedback: sections::feedback::FeedbackState,
    pub overlays: sections::overlays::OverlaysState,
    pub data: sections::data::DataState,
    pub files: sections::files::FilesState,
    pub board: sections::board::BoardState,
    pub date: sections::date::DateState,
    pub chat: sections::chat::ChatState,
    pub code: sections::code::CodeState,
    pub term: sections::term::TermState,
    pub effects: sections::effects::EffectsState,
}

impl Gallery {
    fn new() -> Gallery {
        Gallery {
            mode: ColorMode::detect(),
            shell: ShellState::new(),
            forms: Default::default(),
            pickers: Default::default(),
            structure: Default::default(),
            feedback: Default::default(),
            overlays: Default::default(),
            data: Default::default(),
            files: Default::default(),
            board: Default::default(),
            date: Default::default(),
            chat: Default::default(),
            code: Default::default(),
            term: Default::default(),
            effects: Default::default(),
        }
    }

    fn section(&self) -> usize {
        self.shell.selected
    }
}

impl App for Gallery {
    fn draw(&mut self, frame: &mut Frame, ctx: &mut Ctx) {
        let t = ctx.theme.clone();
        let nav_focused = ctx.focus.register(NAV);
        let nav_sections = [
            NavSection::new(Some("Basics"), &SECTIONS[0..2]),
            NavSection::new(Some("Forms"), &SECTIONS[2..4]),
            NavSection::new(Some("Structure"), &SECTIONS[4..6]),
            NavSection::new(Some("Data"), &SECTIONS[6..9]),
            NavSection::new(Some("Viz"), &SECTIONS[9..11]),
            NavSection::new(Some("Specialty"), &SECTIONS[11..17]),
        ];
        let shell = AppShell::new("◆ FORGE", &nav_sections)
            .subtitle("tui gallery")
            .topbar(SECTIONS[self.section()])
            .topbar_right("alt-ai@wiltaylor.dev")
            .status("Tab focus · ↑/↓ section · t theme · q quit")
            .status_right("forge-tui 0.1")
            .nav_focused(nav_focused)
            .theme(&t);
        frame.render_stateful_widget(shell, frame.area(), &mut self.shell);
        let content = self.shell.content();

        match self.section() {
            0 => sections::primitives::draw(frame, content, ctx, &t),
            1 => sections::feedback::draw(frame, content, ctx, &t, &mut self.feedback),
            2 => sections::forms::draw(frame, content, ctx, &t, &mut self.forms),
            3 => sections::pickers::draw(frame, content, ctx, &t, &mut self.pickers),
            4 => sections::structure::draw(frame, content, ctx, &t, &mut self.structure),
            5 => sections::overlays::draw(frame, content, ctx, &t, &mut self.overlays),
            6 => sections::data::draw(frame, content, ctx, &t, &mut self.data),
            7 => sections::files::draw(frame, content, ctx, &t, &mut self.files),
            8 => sections::board::draw(frame, content, ctx, &t, &mut self.board),
            9 => sections::charts::draw(frame, content, ctx, &t),
            10 => sections::date::draw(frame, content, ctx, &t, &mut self.date),
            11 => sections::text::draw(frame, content, ctx, &t),
            12 => sections::chat::draw(frame, content, ctx, &t, &mut self.chat),
            13 => sections::code::draw(frame, content, ctx, &t, &mut self.code),
            14 => sections::term::draw(frame, content, ctx, &t, &mut self.term),
            15 => sections::flow::draw(frame, content, ctx, &t),
            _ => sections::effects::draw(frame, content, ctx, &t, &mut self.effects),
        }
    }

    fn on_event(&mut self, event: Event, ctx: &mut Ctx) {
        match event {
            Event::Key(key) => {
                let focused = ctx.focus.current();
                let outcome = if focused == Some(NAV) {
                    self.shell.handle_key(key)
                } else {
                    match self.section() {
                        1 => self.feedback.handle_key(focused, key, ctx),
                        2 => self.forms.handle_key(focused, key, ctx),
                        3 => self.pickers.handle_key(focused, key, ctx),
                        4 => self.structure.handle_key(focused, key),
                        5 => self.overlays.handle_key(focused, key, ctx),
                        6 => self.data.handle_key(focused, key, ctx),
                        7 => self.files.handle_key(focused, key, ctx),
                        8 => self.board.handle_key(focused, key, ctx),
                        10 => self.date.handle_key(focused, key, ctx),
                        12 => self.chat.handle_key(focused, key, ctx),
                        13 => self.code.handle_key(focused, key),
                        14 => self.term.handle_key(focused, key, ctx),
                        16 => self.effects.handle_key(focused, key, ctx),
                        _ => Outcome::Ignored,
                    }
                };
                if outcome.is_handled() {
                    return;
                }
                if is_press(&key) && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if key.code == KeyCode::Char('b') {
                        let _ = self.shell.handle_key(key);
                        return;
                    }
                    if key.code == KeyCode::Char('k') {
                        self.overlays.open_palette(ctx);
                        return;
                    }
                }
                match key.code {
                    KeyCode::Char('q') => ctx.quit(),
                    KeyCode::Char('?') => self.overlays.open_help(ctx),
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
            Event::Mouse(ev) => {
                if self.shell.handle_mouse(&ev).is_handled() {
                    return;
                }
                let _ = match self.section() {
                    1 => self.feedback.handle_mouse(&ev, ctx),
                    2 => self.forms.handle_mouse(&ev, ctx),
                    3 => self.pickers.handle_mouse(&ev, ctx),
                    4 => self.structure.handle_mouse(&ev, ctx),
                    5 => self.overlays.handle_mouse(&ev, ctx),
                    6 => self.data.handle_mouse(&ev, ctx),
                    7 => self.files.handle_mouse(&ev, ctx),
                    8 => self.board.handle_mouse(&ev, ctx),
                    10 => self.date.handle_mouse(&ev, ctx),
                    12 => self.chat.handle_mouse(&ev, ctx),
                    13 => self.code.handle_mouse(&ev, ctx),
                    16 => self.effects.handle_mouse(&ev, ctx),
                    _ => Outcome::Ignored,
                };
            }
            Event::Paste(text) => match self.section() {
                2 => self.forms.paste(ctx.focus.current(), &text),
                3 => self.pickers.paste(ctx.focus.current(), &text),
                _ => {}
            },
            _ => {}
        }
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        self.overlays.poll_results(ctx);
        self.term.tick();
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
            app.shell.selected = section;
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
        render_all(100, 30);
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
            app.shell.selected = section;
            let mut ctx = forge_tui::runtime::test_ctx(Theme::dark());
            let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
            terminal
                .draw(|frame| {
                    ctx.focus.begin_frame();
                    app.draw(frame, &mut ctx);
                })
                .unwrap();
            println!("═══ {} ═══", SECTIONS[section]);
            let buf = terminal.backend().buffer();
            for y in 0..32u16 {
                let line: String = (0..100u16).map(|x| buf[(x, y)].symbol()).collect();
                println!("{}", line.trim_end());
            }
        }
    }
}
