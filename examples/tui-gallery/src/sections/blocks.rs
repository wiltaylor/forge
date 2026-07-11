use forge_tui::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

const EDITOR: FocusId = FocusId::new("bl-editor");

/// Demo custom block: a counter box proving the `CustomBlock` trait — `+`/`-`
/// mutate `data["count"]` while the block is entered.
struct CounterBlock;

impl CustomBlock for CounterBlock {
    fn kind(&self) -> &'static str {
        "counter"
    }

    fn label(&self) -> &'static str {
        "Counter"
    }

    fn default_data(&self) -> serde_json::Value {
        serde_json::json!({ "count": 0 })
    }

    fn height(&self, _data: &serde_json::Value, _width: u16, _t: &Theme) -> u16 {
        3
    }

    fn render(
        &mut self,
        data: &serde_json::Value,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        t: &Theme,
    ) {
        if area.width < 4 || area.height < 3 {
            return;
        }
        let count = data.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
        let edge = if focused {
            t.accent.base
        } else {
            t.border.default
        };
        let w = area.width as usize;
        let line = Style::new().fg(edge);
        buf.set_string(area.x, area.y, format!("┌{}┐", "─".repeat(w - 2)), line);
        buf.set_string(area.x, area.y + 1, "│", line);
        buf.set_string(area.x + area.width - 1, area.y + 1, "│", line);
        buf.set_string(area.x, area.y + 2, format!("└{}┘", "─".repeat(w - 2)), line);
        let label = format!("── count: {count} ──  (+/- adjust)");
        buf.set_string(
            area.x + 2,
            area.y + 1,
            forge_tui::text::truncate(&label, w.saturating_sub(4)),
            Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD),
        );
    }

    fn handle_key(&mut self, data: &mut serde_json::Value, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let delta = match key.code {
            KeyCode::Char('+') | KeyCode::Char('=') => 1,
            KeyCode::Char('-') => -1,
            _ => return Outcome::Ignored,
        };
        let count = data.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
        data["count"] = serde_json::json!(count + delta);
        Outcome::Changed
    }
}

pub struct BlocksState {
    pub editor: BlockEditorState,
}

impl Default for BlocksState {
    fn default() -> BlocksState {
        let mut editor = BlockEditorState::new(forge_blocks::sample::sample_document());
        editor.register_custom(Box::new(CounterBlock));
        BlocksState { editor }
    }
}

impl BlocksState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent) -> Outcome {
        match focused {
            Some(id) if id == EDITOR => self.editor.handle_key(key),
            _ => Outcome::Ignored,
        }
    }

    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        let out = self.editor.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(EDITOR);
        }
        out
    }

    pub fn paste(&mut self, focused: Option<FocusId>, text: &str) {
        if focused == Some(EDITOR) {
            let _ = self.editor.paste(text);
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut BlocksState) {
    let focused = ctx.focus.register(EDITOR);
    if area.height < 4 {
        return;
    }
    frame.render_stateful_widget(
        BlockEditor::new().focused(focused).theme(t),
        Rect::new(area.x, area.y, area.width, area.height - 1),
        &mut state.editor,
    );
    let help = "Enter edit · Esc select · / palette on empty · :emoji · Tab indent · Alt+↑/↓ move · c columns · Ctrl+T tone";
    frame.render_widget(
        Line::from(Span::styled(
            forge_tui::text::truncate(help, area.width as usize).into_owned(),
            Style::new().fg(t.fg[2]),
        )),
        Rect::new(area.x, area.y + area.height - 1, area.width, 1),
    );
}
