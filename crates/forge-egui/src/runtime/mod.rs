//! The optional app runtime: one `run()` call gives you the Forge app frame —
//! themed window, [`Shell`] chrome, toasts, dialogs — mirroring
//! `forge_tui::runtime`. Widgets never depend on this module; any eframe app
//! works once [`Theme::apply`](crate::theme::Theme::apply) has run.

mod dialogs;
mod fx;
mod shell;
mod toaster;

pub use dialogs::{Command, DialogResult};
pub use fx::{FxHandle, Motion};
pub use shell::{NavItem, NavSection, Shell, ShellState};
pub use toaster::ToastHandle;

use crate::theme::Theme;

/// A forge-egui application. `ui` runs every frame inside the themed root;
/// `tick` runs first with the frame's delta time (animations, polling
/// [`DialogResult`]s, draining background work).
pub trait App {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx);
    fn tick(&mut self, _dt: f32, _ctx: &mut Ctx) {}
}

pub struct RunOptions {
    pub window_title: String,
    pub initial_size: egui::Vec2,
    pub min_size: egui::Vec2,
    /// Particle-FX motion preference (`Auto` reads `FORGE_EGUI_MOTION`).
    pub motion: Motion,
    /// Escape hatch: full eframe passthrough. The viewport title/size fields
    /// above are applied on top of this.
    pub native: eframe::NativeOptions,
}

impl Default for RunOptions {
    fn default() -> Self {
        RunOptions {
            window_title: "Forge".to_owned(),
            initial_size: egui::vec2(1200.0, 800.0),
            min_size: egui::vec2(720.0, 480.0),
            motion: Motion::Auto,
            native: eframe::NativeOptions::default(),
        }
    }
}

/// Per-frame context handed to [`App`] callbacks: theme access, toasts,
/// dialogs, quit.
pub struct Ctx {
    egui: egui::Context,
    theme: Theme,
    toaster: toaster::Toaster,
    dialogs: dialogs::DialogHost,
    fx: fx::FxEngine,
    quit: bool,
}

impl Ctx {
    fn new(egui: egui::Context, theme: Theme, motion: Motion) -> Ctx {
        let toaster = toaster::Toaster::new(egui.clone());
        let fx = fx::FxEngine::new(egui.clone(), motion);
        Ctx {
            egui,
            theme,
            toaster,
            dialogs: dialogs::DialogHost::default(),
            fx,
            quit: false,
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Swap the theme at runtime (re-applies to the egui context).
    pub fn set_theme(&mut self, theme: Theme) {
        theme.apply(&self.egui);
        self.theme = theme;
    }

    /// A `Clone + Send` handle for pushing toasts from any thread.
    pub fn toast(&self) -> ToastHandle {
        self.toaster.handle()
    }

    /// A `Clone + Send` handle for particle effects.
    pub fn fx(&self) -> FxHandle {
        self.fx.handle()
    }

    /// Any live particles inside `rect` — defer removing an exploding
    /// element until this goes false.
    pub fn fx_active_in(&self, rect: egui::Rect) -> bool {
        self.fx.active_in(rect)
    }

    /// Ask a yes/no question; poll the result in `tick`/`ui` with
    /// [`DialogResult::take`].
    pub fn confirm(&mut self, title: &str, message: &str) -> DialogResult<bool> {
        self.dialogs.confirm(title, message, "Confirm", false)
    }

    /// [`Ctx::confirm`] with a danger-styled verb for destructive actions.
    pub fn confirm_danger(&mut self, title: &str, message: &str, verb: &str) -> DialogResult<bool> {
        self.dialogs.confirm(title, message, verb, true)
    }

    /// Open the command palette over `commands`; resolves with the chosen index.
    pub fn open_palette(&mut self, commands: Vec<Command>) -> DialogResult<usize> {
        self.dialogs.palette(commands)
    }

    /// True while a modal dialog (confirm, palette) is open — apps should
    /// skip their own global shortcuts while this holds.
    pub fn dialog_open(&self) -> bool {
        self.dialogs.is_open()
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn egui(&self) -> &egui::Context {
        &self.egui
    }
}

/// Build a [`Ctx`] without a window — for headless tests (mirrors
/// forge-tui's `test_ctx`).
pub fn test_ctx(theme: Theme) -> Ctx {
    let egui = egui::Context::default();
    theme.apply(&egui);
    Ctx::new(egui, theme, Motion::Off)
}

struct Runner<A: App> {
    app: A,
    ctx: Ctx,
}

impl<A: App> eframe::App for Runner<A> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let dt = ui.input(|i| i.stable_dt).min(0.1);

        self.app.tick(dt, &mut self.ctx);

        // Root background, then the app.
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, self.ctx.theme.bg[0]);
        self.app.ui(ui, &mut self.ctx);

        // Chrome above the app: fx under dialogs, toasts topmost.
        let egui_ctx = ui.ctx().clone();
        let theme = self.ctx.theme.clone();
        self.ctx.fx.step_and_paint(&egui_ctx, &theme, dt);
        self.ctx.dialogs.show(&egui_ctx, &theme);
        self.ctx.toaster.show(&egui_ctx, &theme);

        if self.ctx.quit {
            egui_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Run a forge-egui app: opens the window, installs the theme, and drives
/// [`App::tick`]/[`App::ui`] with the runtime chrome (dialogs, toasts).
pub fn run(app: impl App + 'static, theme: Theme, opts: RunOptions) -> crate::Result<()> {
    let motion = opts.motion;
    let mut native = opts.native;
    native.viewport = native
        .viewport
        .with_title(opts.window_title.clone())
        .with_inner_size(opts.initial_size)
        .with_min_inner_size(opts.min_size);
    eframe::run_native(
        &format!(
            "forge-{}",
            opts.window_title.to_lowercase().replace(' ', "-")
        ),
        native,
        Box::new(move |cc| {
            theme.apply(&cc.egui_ctx);
            let ctx = Ctx::new(cc.egui_ctx.clone(), theme, motion);
            Ok(Box::new(Runner { app, ctx }))
        }),
    )?;
    Ok(())
}
