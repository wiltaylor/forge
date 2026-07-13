//! Cell-glyph particle effects, mirroring the web kit's `fx` module: a region
//! of the screen "explodes" into its own glyphs, converges back together
//! (recreate), materializes in from outside, or emits a celebratory sparkle —
//! all without the app removing anything. The effects layer samples the cells
//! the app just drew, paints over the region while the effect runs, and stops;
//! the app's normal redraw takes over from there.
//!
//! Follows the [`Toaster`](super::toaster::Toaster) shape: any thread pushes
//! requests through a cloned [`FxHandle`]; the runtime advances physics in
//! `tick()` and samples/paints in `draw()` (called before overlays/toasts, so
//! particles fly under dialog scrims).
//!
//! Accessibility & performance: the requested [`Motion`] resolves once at
//! startup (`FORGE_TUI_MOTION` env → explicit option → environment
//! heuristics), and a per-tick timing monitor degrades or fast-forwards
//! effects when the loop can't keep up. Reduced motion turns every effect
//! into a two-tick dim-flash; `Off` makes them instant no-ops — callers never
//! branch.

use crate::theme::color::{approx_rgb, quantize};
use crate::theme::{blend, ColorMode, Theme};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::Frame;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

/// Motion preference for particle effects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    /// Resolve from the environment: `TERM=dumb`, 16-color terminals,
    /// `NO_COLOR`, or a tick rate above 250ms all select [`Motion::Reduced`].
    #[default]
    Auto,
    Full,
    /// Effects become a brief dim-flash of the region — no particles.
    Reduced,
    /// Effects complete instantly; the API still "works".
    Off,
}

impl Motion {
    /// Resolve `Auto` (and apply the `FORGE_TUI_MOTION` override) against the
    /// detected terminal capabilities. Called once by the runtime at startup.
    pub fn resolve(self, mode: ColorMode, tick_rate: Duration) -> Motion {
        if let Ok(v) = std::env::var("FORGE_TUI_MOTION") {
            match v.to_ascii_lowercase().as_str() {
                "full" => return Motion::Full,
                "reduced" | "reduce" => return Motion::Reduced,
                "off" | "none" => return Motion::Off,
                _ => {}
            }
        }
        if self != Motion::Auto {
            return self;
        }
        let dumb = matches!(std::env::var("TERM").as_deref(), Ok("dumb"));
        let no_color = std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());
        if dumb || no_color || mode == ColorMode::Ansi16 || tick_rate > Duration::from_millis(250) {
            Motion::Reduced
        } else {
            Motion::Full
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FxKind {
    Explode,
    Recreate,
    Materialize,
    Sparkle,
}

impl FxKind {
    fn duration_ms(self) -> u64 {
        match self {
            FxKind::Explode => 700,
            FxKind::Recreate => 1100,
            FxKind::Materialize => 600,
            FxKind::Sparkle => 900,
        }
    }

    fn hides(self) -> bool {
        self != FxKind::Sparkle
    }
}

#[derive(Clone, Copy, Debug)]
struct FxRequest {
    kind: FxKind,
    rect: Rect,
}

/// Cheap `Clone + Send` handle for requesting effects from anywhere.
#[derive(Clone)]
pub struct FxHandle {
    tx: Sender<FxRequest>,
}

impl FxHandle {
    /// The region bursts into its own glyphs and scatters.
    pub fn explode(&self, rect: Rect) {
        let _ = self.tx.send(FxRequest {
            kind: FxKind::Explode,
            rect,
        });
    }

    /// Explode, then the glyphs fly back to their home cells.
    pub fn recreate(&self, rect: Rect) {
        let _ = self.tx.send(FxRequest {
            kind: FxKind::Recreate,
            rect,
        });
    }

    /// Glyphs converge into the region from outside.
    pub fn materialize(&self, rect: Rect) {
        let _ = self.tx.send(FxRequest {
            kind: FxKind::Materialize,
            rect,
        });
    }

    /// Celebratory glyph burst over the region; content stays visible.
    pub fn sparkle(&self, rect: Rect) {
        let _ = self.tx.send(FxRequest {
            kind: FxKind::Sparkle,
            rect,
        });
    }
}

/// xorshift64* — tiny deterministic RNG, no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32() * (hi - lo)
    }
}

const SPARKLE_GLYPHS: [char; 5] = ['*', '+', '·', '✦', '✧'];
/// Terminal cells are ~2× taller than wide — squash vertical velocity so
/// bursts look circular instead of egg-shaped.
const ASPECT: f32 = 0.55;
const GRAVITY: f32 = 0.18;
const DRAG: f32 = 0.92;
const MAX_PARTICLES_PER_EFFECT: usize = 1024;
const MAX_PARTICLES_TOTAL: usize = 4096;

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    home: (u16, u16),
    symbol: char,
    fg: Color,
    base_rgb: Option<(u8, u8, u8)>,
    delay: u16,
}

struct Effect {
    kind: FxKind,
    rect: Rect,
    /// Buffer size at sample time — a resize invalidates every coordinate.
    screen: (u16, u16),
    age: u16,
    len: u16,
    /// Recreate: tick where scatter flips to converge.
    converge_at: u16,
    /// Reduced motion: no particles, just a dim-flash.
    reduced: bool,
    particles: Vec<Particle>,
}

impl Effect {
    fn finished(&self) -> bool {
        self.age >= self.len
    }
}

/// The effects manager. Owned by the runtime `Ctx`; apps talk to it through
/// [`FxHandle`] (`ctx.fx()`).
pub struct Fx {
    tx: Sender<FxRequest>,
    rx: Receiver<FxRequest>,
    pending: Vec<FxRequest>,
    active: Vec<Effect>,
    rng: Rng,
    motion: Motion,
    tick_rate: Duration,
    mode: ColorMode,
    last_tick: Option<Instant>,
    ema_ms: f32,
    over_budget: u8,
    degraded: bool,
}

impl Default for Fx {
    fn default() -> Fx {
        Fx::new()
    }
}

impl Fx {
    pub fn new() -> Fx {
        Fx::with_seed(0x243F_6A88_85A3_08D3)
    }

    /// Deterministic RNG seed — for tests and snapshots.
    pub fn with_seed(seed: u64) -> Fx {
        let (tx, rx) = channel();
        Fx {
            tx,
            rx,
            pending: Vec::new(),
            active: Vec::new(),
            rng: Rng::new(seed),
            motion: Motion::Full,
            tick_rate: Duration::from_millis(80),
            mode: ColorMode::TrueColor,
            last_tick: None,
            ema_ms: 0.0,
            over_budget: 0,
            degraded: false,
        }
    }

    /// Late configuration once the terminal is known. The runtime calls this
    /// from `run()`; call it yourself when driving `Fx` in your own loop.
    pub fn configure(&mut self, tick_rate: Duration, motion: Motion, mode: ColorMode) {
        self.tick_rate = tick_rate;
        self.motion = motion.resolve(mode, tick_rate);
        self.mode = mode;
    }

    pub fn handle(&self) -> FxHandle {
        FxHandle {
            tx: self.tx.clone(),
        }
    }

    pub fn is_idle(&self) -> bool {
        self.active.is_empty() && self.pending.is_empty()
    }

    /// Whether any running effect touches `rect` — lets an app defer actually
    /// removing an item until its explosion has finished.
    pub fn active_in(&self, rect: Rect) -> bool {
        self.active.iter().any(|e| e.rect.intersects(rect))
            || self.pending.iter().any(|r| r.rect.intersects(rect))
    }

    /// Advance physics and the performance monitor. Runtime tick only.
    pub fn tick(&mut self) {
        self.measure();
        for effect in &mut self.active {
            effect.age += 1;
            if effect.reduced {
                continue;
            }
            let converging = effect.kind == FxKind::Materialize || effect.age >= effect.converge_at;
            let last = effect.age + 1 >= effect.len;
            for p in &mut effect.particles {
                if p.delay > 0 {
                    p.delay -= 1;
                    continue;
                }
                if converging && effect.kind != FxKind::Explode && effect.kind != FxKind::Sparkle {
                    let hx = p.home.0 as f32;
                    let hy = p.home.1 as f32;
                    p.x += (hx - p.x) * 0.35;
                    p.y += (hy - p.y) * 0.35;
                    if last || ((hx - p.x).abs() < 0.5 && (hy - p.y).abs() < 0.5) {
                        p.x = hx;
                        p.y = hy;
                    }
                } else {
                    p.vy += GRAVITY;
                    p.vx *= DRAG;
                    p.vy *= DRAG;
                    p.x += p.vx;
                    p.y += p.vy;
                }
            }
        }
        self.active.retain(|e| !e.finished());
    }

    /// Track real tick spacing; degrade when the loop can't keep up.
    fn measure(&mut self) {
        let now = Instant::now();
        if let Some(prev) = self.last_tick.replace(now) {
            let dt_ms = now.duration_since(prev).as_secs_f32() * 1000.0;
            let target = self.tick_rate.as_secs_f32() * 1000.0;
            self.ema_ms = if self.ema_ms == 0.0 {
                dt_ms
            } else {
                self.ema_ms * 0.8 + dt_ms * 0.2
            };
            if self.ema_ms > target * 2.5 {
                // Severely behind: finish everything now, spawn reduced.
                for e in &mut self.active {
                    e.age = e.len;
                }
                self.degraded = true;
            } else if self.ema_ms > target * 1.5 {
                self.over_budget = self.over_budget.saturating_add(1);
                if self.over_budget >= 8 {
                    self.degraded = true;
                }
            } else {
                self.over_budget = 0;
                self.degraded = false;
            }
        }
    }

    fn live_particles(&self) -> usize {
        self.active.iter().map(|e| e.particles.len()).sum()
    }

    fn ticks_for(&self, ms: u64) -> u16 {
        let tick_ms = self.tick_rate.as_millis().max(1) as u64;
        (ms / tick_ms).max(2) as u16
    }

    /// Drain requests (sampling the freshly drawn buffer), paint over hidden
    /// regions, and paint particles. Called by the runtime after `App::draw`
    /// and before overlays/toasts.
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        while let Ok(req) = self.rx.try_recv() {
            self.pending.push(req);
        }
        if self.motion == Motion::Off {
            self.pending.clear();
        }

        let screen = (area.width, area.height);
        // A resize invalidates sampled coordinates — finish those effects now.
        self.active.retain(|e| e.screen == screen);

        let requests: Vec<FxRequest> = self.pending.drain(..).collect();
        for req in requests {
            self.spawn(req, frame, area, theme);
        }

        let bg = theme.bg[0];
        let page_rgb = approx_rgb(bg);
        let buf = frame.buffer_mut();

        for effect in &self.active {
            if effect.reduced {
                if let Some(rect) = intersect(effect.rect, area) {
                    buf.set_style(rect, Style::new().fg(theme.fg[3]).bg(bg));
                }
                continue;
            }
            if effect.kind.hides() {
                if let Some(rect) = intersect(effect.rect, area) {
                    for y in rect.top()..rect.bottom() {
                        for x in rect.left()..rect.right() {
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_char(' ');
                                cell.set_fg(bg);
                                cell.set_bg(bg);
                            }
                        }
                    }
                }
            }

            let t = effect.age as f32 / effect.len as f32;
            for p in &effect.particles {
                if p.delay > 0 {
                    continue;
                }
                let x = p.x.round();
                let y = p.y.round();
                if x < 0.0 || y < 0.0 || x >= area.width as f32 || y >= area.height as f32 {
                    continue;
                }
                let fg = self.faded(p, effect, t, page_rgb, theme);
                if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
                    cell.set_char(p.symbol);
                    cell.set_fg(fg);
                }
            }
        }
    }

    /// Fade a particle's color toward the page background over its life:
    /// out while scattering, back in while converging. Terminals without a
    /// recoverable RGB (ANSI-16) switch to the dimmest fg for the tail end.
    fn faded(
        &self,
        p: &Particle,
        effect: &Effect,
        t: f32,
        page_rgb: Option<(u8, u8, u8)>,
        theme: &Theme,
    ) -> Color {
        let alpha = match effect.kind {
            FxKind::Explode | FxKind::Sparkle => 1.0 - t,
            FxKind::Materialize => t,
            FxKind::Recreate => {
                let flip = effect.converge_at as f32 / effect.len as f32;
                if t < flip {
                    1.0 - (t / flip) * 0.7
                } else {
                    0.3 + ((t - flip) / (1.0 - flip)) * 0.7
                }
            }
        };
        match (p.base_rgb, page_rgb) {
            (Some((r, g, b)), Some(bg)) => quantize(
                blend(Color::Rgb(r, g, b), Color::Rgb(bg.0, bg.1, bg.2), alpha),
                self.mode,
            ),
            _ => {
                if alpha < 0.35 {
                    theme.fg[3]
                } else {
                    p.fg
                }
            }
        }
    }

    fn spawn(&mut self, req: FxRequest, frame: &mut Frame, area: Rect, theme: &Theme) {
        let Some(rect) = intersect(req.rect, area) else {
            return;
        };
        let screen = (area.width, area.height);
        let len = self.ticks_for(req.kind.duration_ms());
        let converge_at = match req.kind {
            FxKind::Recreate => ((len as f32 * 0.45) as u16).max(1),
            _ => 0,
        };

        let over_cap = self.live_particles() >= MAX_PARTICLES_TOTAL;
        if self.motion == Motion::Reduced || (self.degraded && self.ema_ms > 0.0) || over_cap {
            self.active.push(Effect {
                kind: req.kind,
                rect,
                screen,
                age: 0,
                len: 2,
                converge_at: 0,
                reduced: true,
                particles: Vec::new(),
            });
            return;
        }

        let cx = rect.x as f32 + rect.width as f32 / 2.0;
        let cy = rect.y as f32 + rect.height as f32 / 2.0;
        let mut particles = Vec::new();

        if req.kind == FxKind::Sparkle {
            let accents = [
                theme.accent.base,
                theme.success.base,
                theme.warning.base,
                theme.info.base,
            ];
            let count = (rect.width as usize * 2).clamp(8, 48);
            for i in 0..count {
                let x = self.rng.range(rect.left() as f32, rect.right() as f32);
                let y = self.rng.range(rect.top() as f32, rect.bottom() as f32);
                let fg = accents[i % accents.len()];
                particles.push(Particle {
                    x,
                    y,
                    vx: self.rng.range(-0.9, 0.9),
                    vy: self.rng.range(-1.4, -0.2) * ASPECT,
                    home: (x as u16, y as u16),
                    symbol: SPARKLE_GLYPHS[i % SPARKLE_GLYPHS.len()],
                    fg,
                    base_rgb: approx_rgb(fg),
                    delay: (self.rng.f32() * 3.0) as u16,
                });
            }
        } else {
            // Sample the region's real glyphs from the freshly drawn buffer.
            let cells = rect.width as usize * rect.height as usize;
            let budget = if self.degraded {
                MAX_PARTICLES_PER_EFFECT / 2
            } else {
                MAX_PARTICLES_PER_EFFECT
            };
            let step = cells.div_ceil(budget).max(1);
            let buf = frame.buffer_mut();
            let mut i = 0usize;
            for y in rect.top()..rect.bottom() {
                for x in rect.left()..rect.right() {
                    i += 1;
                    if !(i - 1).is_multiple_of(step) {
                        continue;
                    }
                    let Some(cell) = buf.cell((x, y)) else {
                        continue;
                    };
                    let sym = cell.symbol();
                    if sym.trim().is_empty() {
                        continue;
                    }
                    // Wide glyphs would clobber a neighbor cell mid-flight —
                    // swap them for a 1-column block.
                    let symbol = if sym.width() > 1 {
                        '▚'
                    } else {
                        sym.chars().next().unwrap_or(' ')
                    };
                    let fg = cell.fg;
                    let (px, py) = if req.kind == FxKind::Materialize {
                        // Spawn scattered on a ring outside the region.
                        let angle = self.rng.f32() * std::f32::consts::TAU;
                        let reach = rect.width.max(rect.height * 2) as f32;
                        let r = reach * self.rng.range(0.7, 1.3) + 2.0;
                        (
                            (cx + angle.cos() * r).clamp(0.0, area.width as f32 - 1.0),
                            (cy + angle.sin() * r * ASPECT).clamp(0.0, area.height as f32 - 1.0),
                        )
                    } else {
                        (x as f32, y as f32)
                    };
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let d = (dx * dx + dy * dy).sqrt().max(1.0);
                    let speed = self.rng.range(0.6, 1.8);
                    particles.push(Particle {
                        x: px,
                        y: py,
                        vx: (dx / d) * speed + self.rng.range(-0.3, 0.3),
                        vy: ((dy / d) * speed + self.rng.range(-0.3, 0.3)) * ASPECT,
                        home: (x, y),
                        symbol,
                        fg,
                        base_rgb: approx_rgb(fg),
                        delay: if req.kind == FxKind::Materialize {
                            (self.rng.f32() * 3.0) as u16
                        } else {
                            0
                        },
                    });
                }
            }
        }

        self.active.push(Effect {
            kind: req.kind,
            rect,
            screen,
            age: 0,
            len,
            converge_at,
            reduced: false,
            particles,
        });
    }
}

fn intersect(rect: Rect, area: Rect) -> Option<Rect> {
    let r = rect.intersection(area);
    (r.width > 0 && r.height > 0).then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;

    #[test]
    fn recreate_particles_land_home() {
        let theme = Theme::dark();
        let mut fx = Fx::with_seed(9);
        fx.configure(
            Duration::from_millis(80),
            Motion::Full,
            ColorMode::TrueColor,
        );
        let rect = Rect::new(5, 5, 10, 3);
        fx.handle().recreate(rect);

        let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(Paragraph::new("##########\n##########\n##########"), rect);
                let area = f.area();
                fx.draw(f, area, &theme);
            })
            .unwrap();
        assert!(!fx.is_idle());
        assert!(fx.active_in(rect));
        let len = fx.active[0].len;
        assert!(!fx.active[0].particles.is_empty());

        // Advance to the last tick: every particle must be back home.
        for _ in 0..len - 1 {
            fx.tick();
        }
        let effect = &fx.active[0];
        for p in &effect.particles {
            assert_eq!((p.x, p.y), (p.home.0 as f32, p.home.1 as f32));
        }
        fx.tick();
        assert!(fx.is_idle());
    }

    #[test]
    fn explode_scatters_and_finishes() {
        let theme = Theme::dark();
        let mut fx = Fx::with_seed(3);
        fx.configure(
            Duration::from_millis(80),
            Motion::Full,
            ColorMode::TrueColor,
        );
        let rect = Rect::new(10, 8, 8, 2);
        fx.handle().explode(rect);

        let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(Paragraph::new("########\n########"), rect);
                let area = f.area();
                fx.draw(f, area, &theme);
            })
            .unwrap();
        for _ in 0..3 {
            fx.tick();
        }
        // Some particles should have left the source region by now.
        let outside = fx.active[0]
            .particles
            .iter()
            .any(|p| !rect.contains(ratatui::layout::Position::new(p.x as u16, p.y as u16)));
        assert!(outside);
        while !fx.is_idle() {
            fx.tick();
        }
    }

    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next(), b.next());
        }
    }

    #[test]
    fn rng_f32_in_unit_range() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.f32();
            assert!((0.0..1.0).contains(&v), "{v}");
        }
    }

    #[test]
    fn ticks_scale_with_tick_rate() {
        let mut fx = Fx::new();
        fx.tick_rate = Duration::from_millis(80);
        assert_eq!(fx.ticks_for(800), 10);
        fx.tick_rate = Duration::from_millis(200);
        assert_eq!(fx.ticks_for(800), 4);
        // Never below two ticks, even at glacial tick rates.
        fx.tick_rate = Duration::from_millis(5000);
        assert_eq!(fx.ticks_for(800), 2);
    }

    // One test for all env-dependent behavior — parallel test threads share
    // the process environment, so these must not run as separate #[test]s.
    #[test]
    fn motion_resolution() {
        std::env::set_var("FORGE_TUI_MOTION", "off");
        let m = Motion::Full.resolve(ColorMode::TrueColor, Duration::from_millis(80));
        std::env::remove_var("FORGE_TUI_MOTION");
        assert_eq!(m, Motion::Off);

        std::env::remove_var("NO_COLOR");
        assert_eq!(
            Motion::Auto.resolve(ColorMode::Ansi16, Duration::from_millis(80)),
            Motion::Reduced
        );
        assert_eq!(
            Motion::Auto.resolve(ColorMode::TrueColor, Duration::from_millis(300)),
            Motion::Reduced
        );
        assert_eq!(
            Motion::Auto.resolve(ColorMode::TrueColor, Duration::from_millis(80)),
            Motion::Full
        );
        // Explicit choice sticks.
        assert_eq!(
            Motion::Full.resolve(ColorMode::Ansi16, Duration::from_millis(80)),
            Motion::Full
        );
    }
}
