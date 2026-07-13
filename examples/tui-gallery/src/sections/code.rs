use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const CODE: FocusId = FocusId::new("cd-code");
const DIFF: FocusId = FocusId::new("cd-diff");

const SOURCE: &str = r#"use forge_tui::prelude::*;

/// Entry point for a Forge console.
fn main() -> forge_tui::Result<()> {
    let mut app = Console::new();
    // TODO: wire real data
    let opts = RunOptions::default();
    forge_tui::runtime::run(&mut app, Theme::dark(), opts)
}

struct Console {
    count: u64,
}

impl Console {
    fn new() -> Console {
        Console { count: 0 }
    }
}
"#;

const OLD: &str = "fn ratio(&self) -> f64 {\n    self.value / self.max\n}\n\nfn label(&self) -> String {\n    format!(\"{}%\", self.ratio() * 100.0)\n}\n";
const NEW: &str = "fn ratio(&self) -> f64 {\n    if self.max <= self.min {\n        return 0.0;\n    }\n    (self.value - self.min) / (self.max - self.min)\n}\n\nfn label(&self) -> String {\n    format!(\"{}%\", self.ratio() * 100.0)\n}\n";

pub struct CodeState {
    pub code: CodeViewState,
    pub diff: CodeViewState,
}

impl Default for CodeState {
    fn default() -> CodeState {
        CodeState {
            code: CodeViewState::new(),
            diff: CodeViewState::new(),
        }
    }
}

impl CodeState {
    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        let out = self.code.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(CODE);
            return out;
        }
        let out = self.diff.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(DIFF);
            return out;
        }
        Outcome::Ignored
    }

    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent) -> Outcome {
        match focused {
            Some(id) if id == CODE => self.code.handle_key(key),
            Some(id) if id == DIFF => self.diff.handle_key(key),
            _ => Outcome::Ignored,
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut CodeState) {
    let f_code = ctx.focus.register(CODE);
    let f_diff = ctx.focus.register(DIFF);
    let cols = Grid::new(2).gap(2).cells(area, 2, area.height);
    if area.height < 4 {
        return;
    }

    frame.render_widget(
        Eyebrow::new("CodeView — gutter marks · scroll").theme(t),
        Rect::new(cols[0].x, cols[0].y, cols[0].width, 1),
    );
    let marks = [(5usize, Severity::Warning), (7usize, Severity::Danger)];
    frame.render_stateful_widget(
        CodeView::new(SOURCE, "rs")
            .marks(&marks)
            .focused(f_code)
            .theme(t),
        Rect::new(cols[0].x, cols[0].y + 1, cols[0].width, cols[0].height - 1),
        &mut state.code,
    );

    frame.render_widget(
        Eyebrow::new("DiffView — built-in line diff").theme(t),
        Rect::new(cols[1].x, cols[1].y, cols[1].width, 1),
    );
    frame.render_stateful_widget(
        DiffView::new(OLD, NEW).theme(t),
        Rect::new(cols[1].x, cols[1].y + 1, cols[1].width, cols[1].height - 1),
        &mut state.diff,
    );
    let _ = f_diff;
}
