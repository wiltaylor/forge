//! Particle-effects integration: sampling/overdraw through a real
//! Terminal<TestBackend> draw cycle, motion gating, and the resize guard.

use forge_tui::runtime::{Fx, Motion};
use forge_tui::theme::{ColorMode, Theme};
use ratatui::backend::TestBackend;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use std::time::Duration;

const RECT: Rect = Rect {
    x: 5,
    y: 5,
    width: 10,
    height: 3,
};

fn fx(motion: Motion) -> Fx {
    fx_at(Duration::from_millis(80), motion)
}

fn fx_at(tick_rate: Duration, motion: Motion) -> Fx {
    let mut fx = Fx::with_seed(42);
    fx.configure(tick_rate, motion, ColorMode::TrueColor);
    fx
}

fn draw_with(terminal: &mut Terminal<TestBackend>, fx: &mut Fx, theme: &Theme) {
    terminal
        .draw(|f| {
            f.render_widget(Paragraph::new("XXXXXXXXXX\nXXXXXXXXXX\nXXXXXXXXXX"), RECT);
            let area = f.area();
            fx.draw(f, area, theme);
        })
        .unwrap();
}

fn count_x(terminal: &Terminal<TestBackend>) -> usize {
    let buf = terminal.backend().buffer();
    let area = buf.area;
    let mut n = 0;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if buf[(x, y)].symbol() == "X" {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn explode_overdraws_region_and_goes_idle() {
    let theme = Theme::dark();
    let mut fx = fx(Motion::Full);
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();

    fx.handle().explode(RECT);
    draw_with(&mut terminal, &mut fx, &theme);
    assert!(!fx.is_idle());
    assert!(fx.active_in(RECT));

    // Mid-flight the region is hidden; surviving glyphs are flying particles,
    // strictly fewer than the 30 the paragraph draws.
    for _ in 0..3 {
        fx.tick();
    }
    draw_with(&mut terminal, &mut fx, &theme);
    assert!(count_x(&terminal) < 30);

    // The scatter is visible: at least one glyph has flown outside the region.
    let buf = terminal.backend().buffer();
    let scattered = buf.area.positions().any(|p| {
        buf[(p.x, p.y)].symbol() == "X" && !RECT.contains(Position::new(p.x, p.y))
    });
    assert!(scattered);

    while !fx.is_idle() {
        fx.tick();
    }
    // Effect over: the app's own redraw shows the content untouched.
    draw_with(&mut terminal, &mut fx, &theme);
    assert_eq!(count_x(&terminal), 30);
}

/// Recreate flies every sampled glyph home: the last frame before the effect
/// goes idle paints the region exactly as the app drew it.
#[test]
fn recreate_lands_every_glyph_home() {
    let theme = Theme::dark();
    let mut fx = fx(Motion::Full);
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();

    fx.handle().recreate(RECT);
    draw_with(&mut terminal, &mut fx, &theme);
    assert!(!fx.is_idle());
    assert!(fx.active_in(RECT));

    // Mid-scatter the effect visibly disturbs the region — the landing check
    // below cannot pass with an effect that never touched the frame.
    for _ in 0..3 {
        fx.tick();
    }
    draw_with(&mut terminal, &mut fx, &theme);
    let buf = terminal.backend().buffer();
    let disturbed = RECT.positions().any(|p| buf[(p.x, p.y)].symbol() != "X");
    assert!(disturbed);

    let mut last_frame = None;
    loop {
        fx.tick();
        if fx.is_idle() {
            break;
        }
        draw_with(&mut terminal, &mut fx, &theme);
        last_frame = Some(terminal.backend().buffer().clone());
    }
    let buf = last_frame.expect("the effect must outlive its first tick");
    for y in RECT.top()..RECT.bottom() {
        for x in RECT.left()..RECT.right() {
            assert_eq!(buf[(x, y)].symbol(), "X", "cell ({x},{y})");
        }
    }
}

/// The effect's length in ticks is derived from the configured tick rate — a
/// recreate spans 1100ms of wall clock however long a tick is — and it never
/// drops below two ticks, even at glacial tick rates.
#[test]
fn effect_length_scales_with_tick_rate() {
    let theme = Theme::dark();
    for (tick_ms, want_ticks) in [(80, 13), (200, 5), (5000, 2)] {
        let mut fx = fx_at(Duration::from_millis(tick_ms), Motion::Full);
        let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();

        fx.handle().recreate(RECT);
        draw_with(&mut terminal, &mut fx, &theme);
        let mut ticks = 0;
        while !fx.is_idle() {
            fx.tick();
            ticks += 1;
            assert!(ticks < 100, "the effect never finished");
        }
        assert_eq!(ticks, want_ticks, "at a {tick_ms}ms tick rate");
    }
}

#[test]
fn sparkle_keeps_content_visible() {
    let theme = Theme::dark();
    let mut fx = fx(Motion::Full);
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();

    fx.handle().sparkle(RECT);
    draw_with(&mut terminal, &mut fx, &theme);
    // Sparkle never hides the region — at most a few cells are covered by
    // sparkle glyphs themselves.
    assert!(count_x(&terminal) > 20);
    assert!(!fx.is_idle());
}

#[test]
fn motion_off_is_instant_noop() {
    let theme = Theme::dark();
    let mut fx = fx(Motion::Off);
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();

    fx.handle().recreate(RECT);
    draw_with(&mut terminal, &mut fx, &theme);
    assert!(fx.is_idle());
    assert_eq!(count_x(&terminal), 30);
}

#[test]
fn motion_reduced_is_brief_dim_flash() {
    let theme = Theme::dark();
    let mut fx = fx(Motion::Reduced);
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();

    fx.handle().explode(RECT);
    draw_with(&mut terminal, &mut fx, &theme);
    assert!(!fx.is_idle());
    // Dim-flash restyles the region but keeps every glyph.
    assert_eq!(count_x(&terminal), 30);

    fx.tick();
    fx.tick();
    assert!(fx.is_idle());
}

#[test]
fn resize_mid_effect_finishes_cleanly() {
    let theme = Theme::dark();
    let mut fx = fx(Motion::Full);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    fx.handle().recreate(RECT);
    draw_with(&mut terminal, &mut fx, &theme);
    assert!(!fx.is_idle());
    fx.tick();

    // Shrink the terminal: sampled coordinates are stale — the next draw must
    // drop the effect without panicking on out-of-bounds cells.
    let mut small = Terminal::new(TestBackend::new(12, 6)).unwrap();
    small
        .draw(|f| {
            let area = f.area();
            fx.draw(f, area, &theme);
        })
        .unwrap();
    assert!(fx.is_idle());
}
