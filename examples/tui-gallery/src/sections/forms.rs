use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const NAME: FocusId = FocusId::new("form-name");
const PASSWORD: FocusId = FocusId::new("form-password");
const HANDLE: FocusId = FocusId::new("form-handle");
const AGREE: FocusId = FocusId::new("form-agree");
const NOTIFY: FocusId = FocusId::new("form-notify");
const LEVEL: FocusId = FocusId::new("form-level");
const SUBMIT: FocusId = FocusId::new("form-submit");

pub struct FormsState {
    pub name: InputState,
    pub password: InputState,
    pub handle: InputState,
    pub agree: CheckboxState,
    pub notify: ToggleState,
    pub level: RadioState,
    submit_rect: Rect,
}

impl Default for FormsState {
    fn default() -> FormsState {
        FormsState {
            name: InputState::with_value("Wil Taylor"),
            password: InputState::new(),
            handle: InputState::with_value("no spaces allowed"),
            agree: CheckboxState::new(true),
            notify: ToggleState::new(false),
            level: RadioState::new(1),
            submit_rect: Rect::default(),
        }
    }
}

impl FormsState {
    fn handle_invalid(&self) -> bool {
        self.handle.value().contains(' ')
    }

    pub fn handle_key(
        &mut self,
        focused: Option<FocusId>,
        key: KeyEvent,
        ctx: &mut Ctx,
    ) -> Outcome {
        let outcome = match focused {
            Some(id) if id == NAME => self.name.handle_key(key),
            Some(id) if id == PASSWORD => self.password.handle_key(key),
            Some(id) if id == HANDLE => self.handle.handle_key(key),
            Some(id) if id == AGREE => self.agree.handle_key(key),
            Some(id) if id == NOTIFY => self.notify.handle_key(key),
            Some(id) if id == LEVEL => self.level.handle_key(key),
            Some(id) if id == SUBMIT => {
                if is_press(&key)
                    && matches!(
                        key.code,
                        ratatui::crossterm::event::KeyCode::Enter
                            | ratatui::crossterm::event::KeyCode::Char(' ')
                    )
                {
                    Outcome::Submitted
                } else {
                    Outcome::Ignored
                }
            }
            _ => Outcome::Ignored,
        };
        match outcome {
            Outcome::Submitted => {
                if self.handle_invalid() {
                    ctx.toast().error("Handle must not contain spaces");
                } else {
                    ctx.toast()
                        .success(format!("Saved profile for {}", self.name.value()));
                }
                Outcome::Consumed
            }
            other => other,
        }
    }

    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        macro_rules! try_widget {
            ($state:expr, $id:expr) => {
                let out = $state.handle_mouse(ev);
                if out.is_handled() {
                    ctx.focus.focus($id);
                    return out;
                }
            };
        }
        try_widget!(self.name, NAME);
        try_widget!(self.password, PASSWORD);
        try_widget!(self.handle, HANDLE);
        try_widget!(self.agree, AGREE);
        try_widget!(self.notify, NOTIFY);
        try_widget!(self.level, LEVEL);
        if forge_tui::event::clicked(ev, self.submit_rect) {
            ctx.focus.focus(SUBMIT);
            if self.handle_invalid() {
                ctx.toast().error("Handle must not contain spaces");
            } else {
                ctx.toast()
                    .success(format!("Saved profile for {}", self.name.value()));
            }
            return Outcome::Consumed;
        }
        Outcome::Ignored
    }

    pub fn paste(&mut self, focused: Option<FocusId>, textv: &str) {
        match focused {
            Some(id) if id == NAME => self.name.insert_str(textv),
            Some(id) if id == PASSWORD => self.password.insert_str(textv),
            Some(id) if id == HANDLE => self.handle.insert_str(textv),
            _ => {}
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut FormsState) {
    let mut y = area.y;
    let x = area.x;
    let w = area.width.min(48);
    let bottom = area.y + area.height;
    let row = |h: u16, gap: u16, y: &mut u16| -> Option<Rect> {
        if *y + h > bottom {
            return None;
        }
        let r = Rect::new(x, *y, w, h);
        *y += h + gap;
        Some(r)
    };

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Profile").theme(t), r);
    }
    if let Some(r) = row(1, 1, &mut y) {
        let focused = ctx.focus.register(NAME);
        frame.render_stateful_widget(
            Input::new()
                .placeholder("Full name")
                .focused(focused)
                .theme(t),
            r,
            &mut state.name,
        );
    }
    if let Some(r) = row(1, 1, &mut y) {
        let focused = ctx.focus.register(PASSWORD);
        frame.render_stateful_widget(
            Input::new()
                .placeholder("Password")
                .masked(true)
                .focused(focused)
                .theme(t),
            r,
            &mut state.password,
        );
    }
    let invalid = state.handle_invalid();
    if let Some(r) = row(1, 0, &mut y) {
        let focused = ctx.focus.register(HANDLE);
        frame.render_stateful_widget(
            Input::new()
                .placeholder("Handle")
                .invalid(invalid)
                .focused(focused)
                .theme(t),
            r,
            &mut state.handle,
        );
    }
    if let Some(r) = row(1, 1, &mut y) {
        if invalid {
            frame.render_widget(
                Badge::new("handle must not contain spaces")
                    .severity(Severity::Danger)
                    .theme(t),
                r,
            );
        }
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Preferences").theme(t), r);
    }
    if let Some(r) = row(1, 0, &mut y) {
        let focused = ctx.focus.register(AGREE);
        frame.render_stateful_widget(
            Checkbox::new("Accept the terms").focused(focused).theme(t),
            r,
            &mut state.agree,
        );
    }
    if let Some(r) = row(1, 1, &mut y) {
        let focused = ctx.focus.register(NOTIFY);
        frame.render_stateful_widget(
            Toggle::new("Email notifications").focused(focused).theme(t),
            r,
            &mut state.notify,
        );
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Log level").theme(t), r);
    }
    if let Some(r) = row(3, 1, &mut y) {
        let focused = ctx.focus.register(LEVEL);
        frame.render_stateful_widget(
            RadioGroup::new(&["debug", "info", "warning"])
                .focused(focused)
                .theme(t),
            r,
            &mut state.level,
        );
    }

    if let Some(r) = row(1, 0, &mut y) {
        let focused = ctx.focus.register(SUBMIT);
        let b = Button::new("Save profile")
            .variant(Variant::Primary)
            .focused(focused)
            .theme(t);
        let bw = b.width().min(r.width);
        state.submit_rect = Rect::new(r.x, r.y, bw, 1);
        frame.render_widget(b, state.submit_rect);
    }
}
