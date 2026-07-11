//! forge-egui backend-integration demo: a native Rust app on forge-core's
//! DocStore/actions/EventBus, all in-process (no HTTP), inside the forge-egui
//! Shell. The Terminal and Desktop pages land with the streaming-widget
//! milestones. Run with `just egui-demo`.

mod backend;
mod pages;

use backend::{Backend, EventFeed};
use forge_egui::prelude::*;

const PAGES: &[&str] = &["Docs", "Actions", "Events", "Terminal", "Desktop"];

struct Demo {
    backend: Backend,
    shell: ShellState,
    feed: Option<EventFeed>,
    shot: Option<self_shot::Shot>,
    docs: pages::docs::DocsPage,
    actions: pages::actions::ActionsPage,
    events: pages::events::EventsPage,
    term: pages::term::TermPage,
    desktop: pages::desktop::DesktopPage,
}

impl Demo {
    fn new() -> Demo {
        let backend = Backend::new();
        // Widget sessions (terminal PTY pumps) spawn onto the backend's
        // runtime. Must happen before the first TermState is constructed;
        // an Err just means a handle is already installed.
        let _ = forge_egui::rt::set_handle(backend.rt.handle().clone());
        let mut shell = ShellState::default();
        // Headless-friendly page selection (see `self_shot`).
        if let Ok(page) = std::env::var("FORGE_DEMO_PAGE") {
            shell.selected = page.parse().unwrap_or(0).min(PAGES.len() - 1);
        }
        Demo {
            backend,
            shell,
            feed: None,
            shot: None,
            docs: Default::default(),
            actions: Default::default(),
            events: Default::default(),
            term: Default::default(),
            desktop: Default::default(),
        }
    }
}

impl App for Demo {
    fn tick(&mut self, _dt: f32, ctx: &mut Ctx) {
        // The forwarder needs the egui context, so it starts on first tick.
        let feed = self
            .feed
            .get_or_insert_with(|| EventFeed::start(&self.backend, ctx.egui()));
        feed.drain();
        self_shot::tick(ctx.egui(), &mut self.shot);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx) {
        let sections = [
            NavSection::new(Some("Backend"), &PAGES[0..3]),
            NavSection::new(Some("Widgets"), &PAGES[3..5]),
        ];
        let shell = Shell::new("◆ FORGE", &sections)
            .subtitle("egui demo")
            .topbar(PAGES[self.shell.selected])
            .status("forge-core in-process · no HTTP")
            .status_right("egui-demo 0.1");
        let selected = self.shell.selected;
        let backend = &self.backend;
        let docs = &mut self.docs;
        let actions = &mut self.actions;
        let events = &mut self.events;
        let term = &mut self.term;
        let desktop = &mut self.desktop;
        let feed = self.feed.as_ref();
        shell.show(ui, &mut self.shell, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match selected {
                0 => docs.ui(ui, backend, ctx),
                1 => actions.ui(ui, backend),
                2 => events.ui(ui, backend, feed),
                3 => term.ui(ui),
                _ => desktop.ui(ui),
            });
        });
    }
}

/// Headless-friendly self-capture for development (the egui-gallery pattern):
/// set `FORGE_DEMO_SHOT` to a PNG path — plus optionally `FORGE_DEMO_PAGE`
/// (page index) and `FORGE_DEMO_SHOT_DELAY` (seconds, default 0.5) — and the
/// demo screenshots itself after the delay, saves, and exits. The delay gives
/// live sessions (VNC/RDP connects) time to render.
mod self_shot {
    use forge_egui::egui;
    use std::time::Instant;

    pub struct Shot {
        start: Instant,
        requested: bool,
    }

    pub fn tick(ctx: &egui::Context, shot: &mut Option<Shot>) {
        let Ok(path) = std::env::var("FORGE_DEMO_SHOT") else {
            return;
        };
        let delay = std::env::var("FORGE_DEMO_SHOT_DELAY")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.5);
        let shot = shot.get_or_insert_with(|| Shot {
            start: Instant::now(),
            requested: false,
        });
        ctx.request_repaint();
        if !shot.requested && shot.start.elapsed().as_secs_f64() >= delay {
            shot.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let pixels: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|p| [p.r(), p.g(), p.b(), p.a()])
                .collect();
            image::save_buffer(&path, &pixels, w as u32, h as u32, image::ColorType::Rgba8)
                .expect("save screenshot");
            std::process::exit(0);
        }
    }
}

fn main() -> forge_egui::Result<()> {
    forge_egui::run(
        Demo::new(),
        Theme::dark(),
        RunOptions {
            window_title: "Forge — egui demo".to_owned(),
            ..Default::default()
        },
    )
}
