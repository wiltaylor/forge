use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const TOAST_BTN: &str = "fb-toast";

#[derive(Default)]
pub struct FeedbackState {
    btn_rects: Vec<Rect>,
}

const TOASTS: [(Severity, &str, &str); 4] = [
    (Severity::Info, "Info", "Deploy queued behind 2 jobs"),
    (Severity::Success, "Success", "forge-tui deployed to prod"),
    (Severity::Warning, "Warning", "Disk usage at 82%"),
    (Severity::Danger, "Error", "Health check failed on node-3"),
];

impl FeedbackState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        if !is_press(&key) || !matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            return Outcome::Ignored;
        }
        for (i, (sev, _, msg)) in TOASTS.iter().enumerate() {
            if focused == Some(FocusId::indexed(TOAST_BTN, i as u32)) {
                ctx.toast().push(*sev, *msg);
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }

    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        for (i, rect) in self.btn_rects.clone().into_iter().enumerate() {
            if forge_tui::event::clicked(ev, rect) {
                ctx.focus.focus(FocusId::indexed(TOAST_BTN, i as u32));
                let (sev, _, msg) = TOASTS[i];
                ctx.toast().push(sev, msg);
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut FeedbackState) {
    state.btn_rects.clear();
    let mut y = area.y;
    let x = area.x;
    let w = area.width.min(56);
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
        frame.render_widget(Eyebrow::new("Alerts").theme(t), r);
    }
    let alerts = [
        (Severity::Info, "Heads up", "A new forge-tui version is available."),
        (Severity::Success, "All systems go", "Deploy finished in 42s."),
        (Severity::Warning, "Degraded", "Event bus is lagging behind."),
        (Severity::Danger, "Down", "forge-auth is unreachable."),
    ];
    for (sev, title, body) in alerts {
        if let Some(r) = row(2, 1, &mut y) {
            frame.render_widget(Alert::new(sev, title).body(body).theme(t), r);
        }
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Progress").theme(t), r);
    }
    if let Some(r) = row(1, 0, &mut y) {
        let ratio = (ctx.frame % 100) as f64 / 100.0;
        frame.render_widget(Progress::new(ratio).label("build").theme(t), r);
    }
    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(
            Progress::new(0.82).label("disk").severity(Severity::Warning).theme(t),
            r,
        );
    }
    if let Some(r) = row(1, 1, &mut y) {
        frame.render_widget(Spinner::new().frame(ctx.frame).label("Deploying…").theme(t), r);
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Toasts").theme(t), r);
    }
    if let Some(r) = row(1, 0, &mut y) {
        let mut bx = r.x;
        for (i, (_, label, _)) in TOASTS.iter().enumerate() {
            let focused = ctx.focus.register(FocusId::indexed(TOAST_BTN, i as u32));
            let b = Button::new(label).focused(focused).theme(t);
            let bw = b.width();
            if bx + bw > r.x + r.width {
                break;
            }
            state.btn_rects.push(Rect::new(bx, r.y, bw, 1));
            frame.render_widget(b, Rect::new(bx, r.y, bw, 1));
            bx += bw + 2;
        }
    }
}
