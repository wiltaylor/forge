use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const VIEW: FocusId = FocusId::new("ch-view");
const PROMPT: FocusId = FocusId::new("ch-prompt");
const COMPOSER: FocusId = FocusId::new("ch-composer");

const PROMPT_OPTIONS: [&str; 3] = ["Deploy now", "Schedule", "Cancel"];

pub struct ChatState {
    pub items: Vec<ChatItem>,
    pub view: ChatViewState,
    pub prompt: ChatPromptState,
    pub composer: ComposerState,
    pub prompt_answered: bool,
}

impl Default for ChatState {
    fn default() -> ChatState {
        let items = vec![
            ChatItem::Divider("today".into()),
            ChatItem::user("Ship the new tui gallery to staging?"),
            ChatItem::ToolCall {
                name: "read_file(justfile)".into(),
                status: ToolStatus::Ok,
                detail: Some("42 recipes · found tui-gallery in group demo".into()),
                open: true,
            },
            ChatItem::assistant(
                "The gallery builds clean. **Plan:**\n\n1. `just tui-test`\n2. Deploy to staging\n3. Smoke-check the shell\n\nWant me to proceed?",
            ),
            ChatItem::Typing("assistant".into()),
        ];
        ChatState {
            items,
            view: ChatViewState::new(),
            prompt: ChatPromptState::new(),
            composer: ComposerState::new(),
            prompt_answered: false,
        }
    }
}

impl ChatState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        let outcome = match focused {
            Some(id) if id == VIEW => self.view.handle_key(key),
            Some(id) if id == PROMPT && !self.prompt_answered => self.prompt.handle_key(key),
            Some(id) if id == COMPOSER => self.composer.handle_key(key),
            _ => Outcome::Ignored,
        };
        match outcome {
            Outcome::Submitted if focused == Some(COMPOSER) => {
                let msg = self.composer.take_message();
                self.items.pop_if_typing();
                self.items.push(ChatItem::user(msg));
                self.items.push(ChatItem::Typing("assistant".into()));
                self.view.follow = true;
                Outcome::Consumed
            }
            Outcome::Submitted if focused == Some(PROMPT) => {
                self.prompt_answered = true;
                ctx.toast().success(format!("Chose: {}", PROMPT_OPTIONS[self.prompt.selected]));
                Outcome::Consumed
            }
            o => o,
        }
    }
}

impl ChatState {
    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        if !self.prompt_answered {
            let out = self.prompt.handle_mouse(ev);
            if out.is_handled() {
                ctx.focus.focus(PROMPT);
                if out == Outcome::Submitted {
                    self.prompt_answered = true;
                    ctx.toast().success(format!("Chose: {}", PROMPT_OPTIONS[self.prompt.selected]));
                    return Outcome::Consumed;
                }
                return out;
            }
        }
        let out = self.view.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(VIEW);
            return out;
        }
        let out = self.composer.input.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(COMPOSER);
            return out;
        }
        Outcome::Ignored
    }
}

trait PopTyping {
    fn pop_if_typing(&mut self);
}

impl PopTyping for Vec<ChatItem> {
    fn pop_if_typing(&mut self) {
        if matches!(self.last(), Some(ChatItem::Typing(_))) {
            self.pop();
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut ChatState) {
    let f_view = ctx.focus.register(VIEW);
    let f_prompt = ctx.focus.register(PROMPT);
    let f_composer = ctx.focus.register(COMPOSER);
    if area.height < 8 {
        return;
    }
    let w = area.width.min(70);
    let prompt_h = if state.prompt_answered { 0 } else { 3 };
    let composer_h = 4;
    let view_h = area.height - prompt_h - composer_h;

    frame.render_stateful_widget(
        ChatView::new(&state.items).frame(ctx.frame).focused(f_view).theme(t),
        Rect::new(area.x, area.y, w, view_h),
        &mut state.view,
    );
    if !state.prompt_answered {
        frame.render_stateful_widget(
            ChatPrompt::new("Ready to deploy?", &PROMPT_OPTIONS).focused(f_prompt).theme(t),
            Rect::new(area.x, area.y + view_h, w, 2),
            &mut state.prompt,
        );
    }
    frame.render_stateful_widget(
        Composer::new().focused(f_composer).theme(t),
        Rect::new(area.x, area.y + view_h + prompt_h, w, composer_h.min(4)),
        &mut state.composer,
    );
}
