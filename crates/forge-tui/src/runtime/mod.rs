//! Optional app runtime: terminal setup/teardown with a panic-safe restore
//! guard, a poll-with-tick event loop, focus traversal, the overlay stack,
//! and the toaster. Widgets never depend on this module — they work in any
//! ratatui app; the runtime is for apps that want the full Forge experience.

pub mod dialogs;
mod focus;
mod overlay;
mod shell;
mod toaster;

pub use dialogs::{
    ConfirmDialog, DialogResult, HelpOverlay, MenuOverlay, OwnedCommand, OwnedMenuEntry,
    PaletteOverlay,
};
pub use focus::{FocusId, FocusRing};
pub use overlay::{dim, Overlay, OverlayOutcome, OverlayStack};
pub use shell::{AppShell, NavSection, ShellState};
pub use toaster::{Toast, ToastHandle, Toaster};

use crate::error::Result;
use crate::event::is_press;
use crate::theme::{set_default_theme, ColorMode, Theme};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEventKind, KeyModifiers,
};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::execute;
use ratatui::{Frame, Terminal};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct RunOptions {
    /// Animation/toast tick rate (drives `App::tick` and `Ctx::frame`).
    pub tick_rate: Duration,
    /// Force a color mode instead of detecting from the environment.
    pub color_mode: Option<ColorMode>,
    /// Capture mouse events (clicks, wheel, hover). On by default — most
    /// terminals still offer native text selection via Shift+drag while
    /// capture is active; set to `false` to restore plain selection.
    pub mouse: bool,
    /// Bracketed paste (`Event::Paste`). On by default.
    pub paste: bool,
}

impl Default for RunOptions {
    fn default() -> RunOptions {
        RunOptions {
            tick_rate: Duration::from_millis(80),
            color_mode: None,
            mouse: true,
            paste: true,
        }
    }
}

/// Per-run context handed to every [`App`] callback.
pub struct Ctx {
    /// The active theme, already quantized for the terminal's color mode.
    pub theme: Theme,
    pub focus: FocusRing,
    pub overlays: OverlayStack,
    toaster: Toaster,
    /// Animation frame counter, advanced once per tick.
    pub frame: u64,
    quit: bool,
}

impl Ctx {
    fn new(theme: Theme) -> Ctx {
        Ctx {
            theme,
            focus: FocusRing::new(),
            overlays: OverlayStack::new(),
            toaster: Toaster::new(),
            frame: 0,
            quit: false,
        }
    }

    /// Ask the runtime to exit after this iteration.
    pub fn quit(&mut self) {
        self.quit = true;
    }

    /// A `Clone + Send` handle for pushing toasts (also from other threads).
    pub fn toast(&self) -> ToastHandle {
        self.toaster.handle()
    }

    /// Dismiss the oldest visible toast.
    pub fn dismiss_toast(&mut self) {
        self.toaster.dismiss();
    }

    /// Open an overlay (modal, sheet, palette, menu…).
    pub fn open(&mut self, overlay: Box<dyn Overlay>) {
        self.overlays.push(overlay);
    }

    fn draw_chrome(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let Ctx { theme, overlays, toaster, .. } = self;
        overlays.draw(frame, area, theme);
        toaster.draw(frame, area, theme);
    }
}

/// Build a [`Ctx`] without a terminal — for tests and snapshot harnesses.
pub fn test_ctx(theme: Theme) -> Ctx {
    Ctx::new(theme)
}

/// A forge-tui application. `draw` runs every frame; `on_event` receives
/// events not consumed by overlays or the global keybinds (Ctrl+C quit,
/// Tab/Shift+Tab focus traversal); `tick` runs at the fixed tick rate.
pub trait App {
    fn draw(&mut self, frame: &mut Frame, ctx: &mut Ctx);
    fn on_event(&mut self, event: Event, ctx: &mut Ctx);
    fn tick(&mut self, _ctx: &mut Ctx) {}
}

static GUARD_ACTIVE: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

fn restore_terminal(mouse: bool, paste: bool) {
    let mut out = io::stdout();
    if mouse {
        let _ = execute!(out, DisableMouseCapture);
    }
    if paste {
        let _ = execute!(out, DisableBracketedPaste);
    }
    let _ = execute!(out, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Raw-mode/alt-screen guard: restores the terminal on drop AND on panic, so
/// a widget bug never leaves the user's shell in raw mode.
struct TerminalGuard {
    mouse: bool,
    paste: bool,
}

impl TerminalGuard {
    fn new(opts: &RunOptions) -> Result<TerminalGuard> {
        PANIC_HOOK.call_once(|| {
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                if GUARD_ACTIVE.load(Ordering::SeqCst) {
                    // Best-effort full restore; flags may not match the run
                    // options but the extra disables are harmless.
                    restore_terminal(true, true);
                }
                prev(info);
            }));
        });
        enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen)?;
        if opts.mouse {
            execute!(out, EnableMouseCapture)?;
        }
        if opts.paste {
            execute!(out, EnableBracketedPaste)?;
        }
        GUARD_ACTIVE.store(true, Ordering::SeqCst);
        Ok(TerminalGuard { mouse: opts.mouse, paste: opts.paste })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        GUARD_ACTIVE.store(false, Ordering::SeqCst);
        restore_terminal(self.mouse, self.paste);
    }
}

/// Run an [`App`] until it calls [`Ctx::quit`] (or the user hits Ctrl+C).
///
/// Detects the terminal's color mode, quantizes the theme once, installs it
/// as the process default, and drives the draw/event/tick loop. Event routing
/// is top-down: topmost overlay → global keybinds → `App::on_event`.
pub fn run(app: &mut dyn App, theme: Theme, opts: RunOptions) -> Result<()> {
    let mode = opts.color_mode.unwrap_or_else(ColorMode::detect);
    let theme = theme.quantized(mode);
    let _ = set_default_theme(theme.clone());

    let guard = TerminalGuard::new(&opts)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut ctx = Ctx::new(theme);
    let mut last_tick = Instant::now();

    while !ctx.quit {
        terminal.draw(|frame| {
            ctx.focus.begin_frame();
            app.draw(frame, &mut ctx);
            ctx.draw_chrome(frame);
        })?;

        let timeout = opts.tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            let ev = event::read()?;
            let release = matches!(&ev, Event::Key(k) if k.kind == KeyEventKind::Release);
            if !release {
                route(app, ev, &mut ctx);
            }
        }
        if last_tick.elapsed() >= opts.tick_rate {
            ctx.frame = ctx.frame.wrapping_add(1);
            ctx.toaster.tick();
            app.tick(&mut ctx);
            last_tick = Instant::now();
        }
    }

    drop(guard);
    Ok(())
}

fn route(app: &mut dyn App, ev: Event, ctx: &mut Ctx) {
    // 1. Overlays are modal: an open overlay swallows everything.
    if ctx.overlays.handle(&ev) {
        return;
    }
    // 2. Global keybinds.
    if let Event::Key(k) = &ev {
        if is_press(k) {
            match k.code {
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    ctx.quit();
                    return;
                }
                KeyCode::Tab if k.modifiers.is_empty() => {
                    ctx.focus.next();
                    return;
                }
                KeyCode::BackTab => {
                    ctx.focus.prev();
                    return;
                }
                _ => {}
            }
        }
    }
    // 3. The app.
    app.on_event(ev, ctx);
}
