//! Glyph-free particle effects — the egui counterpart of `@forge/ui`'s
//! `fx.explode/recreate/materialize/burst` and forge-tui's `FxHandle`.
//!
//! Requests are pushed through a `Clone + Send` [`FxHandle`] (any thread);
//! the runtime steps physics with real frame dt and paints on
//! [`egui::Order::Foreground`], under dialogs and toasts.
//!
//! Divergence from the TUI (documented in the plan): egui cannot cheaply
//! sample rendered pixels, so each request carries an [`FxColors`] source —
//! theme-derived by default, or explicit colors from the caller.

use crate::theme::Theme;
use egui::{Color32, Pos2, Rect, Vec2};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

const MAX_PER_EFFECT: usize = 1024;
const MAX_TOTAL: usize = 4096;

/// Motion preference: resolved once at startup from `FORGE_EGUI_MOTION`
/// (`full` / `reduced` / `off`). No OS reduced-motion signal is exposed by
/// egui/winit, so `Auto` means `Full` unless the env says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Auto,
    Full,
    Reduced,
    Off,
}

impl Motion {
    pub(crate) fn resolve(self) -> Motion {
        match self {
            Motion::Auto => match std::env::var("FORGE_EGUI_MOTION").as_deref() {
                Ok("off") => Motion::Off,
                Ok("reduced") => Motion::Reduced,
                _ => Motion::Full,
            },
            other => other,
        }
    }
}

/// Where an effect's particles get their colors.
#[derive(Clone, Debug, Default)]
pub enum FxColors {
    /// Accent + semantic sparkle mix from the installed theme.
    #[default]
    Theme,
    Explicit(Vec<Color32>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FxKind {
    Explode,
    Recreate,
    Materialize,
    Sparkle,
}

struct FxRequest {
    kind: FxKind,
    rect: Rect,
    colors: FxColors,
}

/// Push effects from anywhere: `fx.explode(rect)`. Cheap to clone; pushes
/// wake the event loop.
#[derive(Clone)]
pub struct FxHandle {
    tx: mpsc::Sender<FxRequest>,
    egui: egui::Context,
}

impl FxHandle {
    fn push(&self, kind: FxKind, rect: Rect, colors: FxColors) {
        let _ = self.tx.send(FxRequest { kind, rect, colors });
        self.egui.request_repaint();
    }

    /// Blow the region apart: particles burst outward and fall.
    pub fn explode(&self, rect: Rect) {
        self.push(FxKind::Explode, rect, FxColors::Theme);
    }

    /// Particles swirl inward — "something appears here".
    pub fn recreate(&self, rect: Rect) {
        self.push(FxKind::Recreate, rect, FxColors::Theme);
    }

    /// Soft build-up: particles rise into place from below.
    pub fn materialize(&self, rect: Rect) {
        self.push(FxKind::Materialize, rect, FxColors::Theme);
    }

    /// Gentle celebratory twinkles over the region.
    pub fn sparkle(&self, rect: Rect) {
        self.push(FxKind::Sparkle, rect, FxColors::Theme);
    }

    /// Any effect with explicit colors.
    pub fn explode_with(&self, rect: Rect, colors: Vec<Color32>) {
        self.push(FxKind::Explode, rect, FxColors::Explicit(colors));
    }

    pub fn sparkle_with(&self, rect: Rect, colors: Vec<Color32>) {
        self.push(FxKind::Sparkle, rect, FxColors::Explicit(colors));
    }
}

struct Particle {
    pos: Pos2,
    vel: Vec2,
    size: f32,
    color: Color32,
    /// Remaining / initial lifetime in seconds.
    life: f32,
    ttl: f32,
    gravity: f32,
    drag: f32,
}

/// A quick flash for `Motion::Reduced`.
struct Flash {
    rect: Rect,
    life: f32,
    color: Color32,
}

/// Deterministic xorshift64* — no `Date::now`/thread RNG so tests can fix
/// the seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in [0, 1).
    fn f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32() * (hi - lo)
    }
}

pub(crate) struct FxEngine {
    tx: mpsc::Sender<FxRequest>,
    rx: mpsc::Receiver<FxRequest>,
    egui: egui::Context,
    motion: Motion,
    particles: Vec<Particle>,
    flashes: Vec<Flash>,
    seed: AtomicU64,
}

impl FxEngine {
    pub(crate) fn new(egui: egui::Context, motion: Motion) -> FxEngine {
        let (tx, rx) = mpsc::channel();
        FxEngine {
            tx,
            rx,
            egui,
            motion: motion.resolve(),
            particles: Vec::new(),
            flashes: Vec::new(),
            seed: AtomicU64::new(0x9E3779B97F4A7C15),
        }
    }

    pub(crate) fn handle(&self) -> FxHandle {
        FxHandle {
            tx: self.tx.clone(),
            egui: self.egui.clone(),
        }
    }

    pub(crate) fn active(&self) -> bool {
        !self.particles.is_empty() || !self.flashes.is_empty()
    }

    /// Any live particle inside `rect` — for callers deferring removal of an
    /// exploding element.
    pub(crate) fn active_in(&self, rect: Rect) -> bool {
        self.flashes.iter().any(|f| f.rect.intersects(rect))
            || self.particles.iter().any(|p| rect.contains(p.pos))
    }

    fn spawn(&mut self, request: FxRequest, theme: &Theme) {
        if self.motion == Motion::Off {
            return;
        }
        let palette: Vec<Color32> = match request.colors {
            FxColors::Explicit(colors) if !colors.is_empty() => colors,
            _ => vec![
                theme.accent.base,
                theme.accent.fg,
                theme.fg[1],
                theme.info.base,
                theme.success.base,
            ],
        };
        if self.motion == Motion::Reduced {
            self.flashes.push(Flash {
                rect: request.rect,
                life: theme.motion.base,
                color: palette[0],
            });
            return;
        }

        let mut rng = Rng(self.seed.fetch_add(0x9E37, Ordering::Relaxed) | 1);
        let rect = request.rect;
        let area = (rect.width() * rect.height()).max(1.0);
        let count = ((area / 60.0) as usize).clamp(24, MAX_PER_EFFECT);
        let budget = MAX_TOTAL.saturating_sub(self.particles.len());
        let count = count.min(budget);

        for _ in 0..count {
            let color = palette[(rng.next() as usize) % palette.len()];
            let inside = Pos2::new(
                rng.range(rect.min.x, rect.max.x),
                rng.range(rect.min.y, rect.max.y),
            );
            let particle = match request.kind {
                FxKind::Explode => {
                    let dir = (inside - rect.center()).normalized();
                    let speed = rng.range(60.0, 240.0);
                    Particle {
                        pos: inside,
                        vel: dir * speed + Vec2::new(0.0, -rng.range(20.0, 90.0)),
                        size: rng.range(1.5, 3.5),
                        color,
                        life: rng.range(0.5, 1.1),
                        ttl: 1.1,
                        gravity: 260.0,
                        drag: 1.2,
                    }
                }
                FxKind::Recreate => {
                    let angle = rng.range(0.0, std::f32::consts::TAU);
                    let radius = rect.size().length() * 0.7;
                    let start = rect.center() + Vec2::angled(angle) * radius;
                    Particle {
                        pos: start,
                        vel: (rect.center() - start) * rng.range(1.6, 2.6),
                        size: rng.range(1.5, 3.0),
                        color,
                        life: rng.range(0.35, 0.6),
                        ttl: 0.6,
                        gravity: 0.0,
                        drag: 2.2,
                    }
                }
                FxKind::Materialize => Particle {
                    pos: Pos2::new(inside.x, rect.max.y + rng.range(4.0, 24.0)),
                    vel: Vec2::new(0.0, -rng.range(40.0, 120.0)),
                    size: rng.range(1.5, 2.5),
                    color,
                    life: rng.range(0.4, 0.8),
                    ttl: 0.8,
                    gravity: -30.0,
                    drag: 1.5,
                },
                FxKind::Sparkle => Particle {
                    pos: inside,
                    vel: Vec2::new(rng.range(-12.0, 12.0), -rng.range(10.0, 40.0)),
                    size: rng.range(1.0, 2.5),
                    color,
                    life: rng.range(0.4, 0.9),
                    ttl: 0.9,
                    gravity: -12.0,
                    drag: 0.6,
                },
            };
            self.particles.push(particle);
        }
    }

    /// Drain requests, step physics, paint. Called once per frame by the
    /// runtime chrome pass.
    pub(crate) fn step_and_paint(&mut self, ctx: &egui::Context, theme: &Theme, dt: f32) {
        while let Ok(request) = self.rx.try_recv() {
            self.spawn(request, theme);
        }
        if !self.active() {
            return;
        }
        let dt = dt.clamp(0.001, 0.05);

        for p in &mut self.particles {
            p.life -= dt;
            p.vel.y += p.gravity * dt;
            p.vel *= 1.0 - (p.drag * dt).min(0.9);
            p.pos += p.vel * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for f in &mut self.flashes {
            f.life -= dt;
        }
        self.flashes.retain(|f| f.life > 0.0);

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("forge-fx"),
        ));
        for f in &self.flashes {
            let alpha = ((f.life / theme.motion.base) * 90.0) as u8;
            painter.rect_filled(
                f.rect,
                egui::CornerRadius::same(theme.radius.md as u8),
                crate::theme::color::with_alpha(f.color, alpha),
            );
        }
        for p in &self.particles {
            let fade = (p.life / p.ttl).clamp(0.0, 1.0);
            let color = crate::theme::color::with_alpha(p.color, (fade * 255.0) as u8);
            painter.rect_filled(
                Rect::from_center_size(p.pos, Vec2::splat(p.size)),
                1.0,
                color,
            );
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(motion: Motion) -> FxEngine {
        FxEngine::new(egui::Context::default(), motion)
    }

    fn step(e: &mut FxEngine, theme: &Theme, frames: usize) {
        let ctx = e.egui.clone();
        for _ in 0..frames {
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                let ictx = ui.ctx().clone();
                e.step_and_paint(&ictx, theme, 1.0 / 60.0);
            });
        }
    }

    #[test]
    fn explode_spawns_then_decays() {
        let theme = Theme::dark();
        let mut e = engine(Motion::Full);
        e.handle()
            .explode(Rect::from_min_size(Pos2::ZERO, egui::vec2(120.0, 40.0)));
        step(&mut e, &theme, 1);
        assert!(e.active());
        let initial = e.particles.len();
        assert!(initial >= 24);
        step(&mut e, &theme, 120); // two simulated seconds
        assert!(!e.active(), "all particles must expire");
    }

    #[test]
    fn fixed_seed_is_deterministic() {
        let mut a = Rng(42);
        let mut b = Rng(42);
        for _ in 0..100 {
            assert_eq!(a.next(), b.next());
        }
    }

    #[test]
    fn reduced_motion_flashes_instead() {
        let theme = Theme::dark();
        let mut e = engine(Motion::Reduced);
        e.handle()
            .explode(Rect::from_min_size(Pos2::ZERO, egui::vec2(50.0, 20.0)));
        step(&mut e, &theme, 1);
        assert!(e.particles.is_empty());
        assert_eq!(e.flashes.len(), 1);
        step(&mut e, &theme, 30);
        assert!(!e.active());
    }

    #[test]
    fn off_motion_is_a_no_op() {
        let theme = Theme::dark();
        let mut e = engine(Motion::Off);
        e.handle()
            .sparkle(Rect::from_min_size(Pos2::ZERO, egui::vec2(50.0, 20.0)));
        step(&mut e, &theme, 1);
        assert!(!e.active());
    }

    #[test]
    fn total_particle_cap_holds() {
        let theme = Theme::dark();
        let mut e = engine(Motion::Full);
        let big = Rect::from_min_size(Pos2::ZERO, egui::vec2(4000.0, 4000.0));
        for _ in 0..10 {
            e.handle().explode(big);
        }
        step(&mut e, &theme, 1);
        assert!(e.particles.len() <= MAX_TOTAL);
    }
}
