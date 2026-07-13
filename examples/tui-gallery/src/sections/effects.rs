use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const FX_BTN: &str = "fx-btn";

const BUTTONS: [&str; 4] = ["Explode", "Recreate", "Materialize", "Sparkle"];

#[derive(Default)]
pub struct EffectsState {
    card: Rect,
    btn_rects: Vec<Rect>,
}

impl EffectsState {
    fn trigger(&self, i: usize, ctx: &mut Ctx) {
        let fx = ctx.fx();
        match i {
            0 => fx.explode(self.card),
            1 => fx.recreate(self.card),
            2 => fx.materialize(self.card),
            _ => fx.sparkle(self.card),
        }
    }

    pub fn handle_key(
        &mut self,
        focused: Option<FocusId>,
        key: KeyEvent,
        ctx: &mut Ctx,
    ) -> Outcome {
        if !is_press(&key) || !matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            return Outcome::Ignored;
        }
        for i in 0..BUTTONS.len() {
            if focused == Some(FocusId::indexed(FX_BTN, i as u32)) {
                self.trigger(i, ctx);
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }

    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        for (i, rect) in self.btn_rects.clone().into_iter().enumerate() {
            if forge_tui::event::clicked(ev, rect) {
                ctx.focus.focus(FocusId::indexed(FX_BTN, i as u32));
                self.trigger(i, ctx);
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut EffectsState) {
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

    if let Some(r) = row(1, 1, &mut y) {
        frame.render_widget(Eyebrow::new("Particle FX").theme(t), r);
    }

    // The demo card — effects sample and overdraw exactly this rect. The card
    // keeps rendering every frame; fx hides it visually while an effect runs.
    if let Some(r) = row(7, 1, &mut y) {
        state.card = r;
        let card = Card::new().title(" prod-worker-04 ").theme(t);
        let inner = card.inner(r);
        frame.render_widget(card, r);
        let lines = vec![
            Line::styled("GPU util        87%   ▂▄▆█▆▄▂", t.fg[0]),
            Line::styled("VRAM         61 GiB   a100 × 8", t.fg[1]),
            Line::styled("status      running   uptime 41d", t.success.base),
            Line::styled("queue depth       3   p95 142ms", t.fg[2]),
            Line::styled("● healthy — last probe 2s ago", t.accent.base),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }

    if let Some(r) = row(1, 1, &mut y) {
        let mut bx = r.x;
        for (i, label) in BUTTONS.iter().enumerate() {
            let focused = ctx.focus.register(FocusId::indexed(FX_BTN, i as u32));
            let variant = if i == 1 {
                Variant::Primary
            } else {
                Variant::Default
            };
            let b = Button::new(label)
                .variant(variant)
                .focused(focused)
                .theme(t);
            let bw = b.width();
            if bx + bw > r.x + r.width {
                break;
            }
            state.btn_rects.push(Rect::new(bx, r.y, bw, 1));
            frame.render_widget(b, Rect::new(bx, r.y, bw, 1));
            bx += bw + 2;
        }
    }

    if let Some(r) = row(2, 0, &mut y) {
        let status = if ctx.fx_idle() { "idle" } else { "running" };
        let lines = vec![
            Line::styled(format!("fx: {status}"), t.fg[2]),
            Line::styled(
                "FORGE_TUI_MOTION=off|reduced|full overrides · auto degrades on dumb/16-color terminals & slow ticks",
                t.fg[3],
            ),
        ];
        frame.render_widget(Paragraph::new(lines), r);
    }
}
