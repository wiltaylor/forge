//! forge-egui gallery — the living catalogue of every widget, mirroring
//! `apps/gallery` (web) and `examples/tui-gallery` (terminal). Run with
//! `just egui-gallery`.

mod sections;

use forge_egui::prelude::*;

pub const SECTIONS: &[&str] = &[
    "Primitives",
    "Feedback",
    "Forms",
    "Pickers",
    "Structure",
    "Overlays",
    "Data",
    "Files",
    "Board",
    "Charts",
    "Date",
    "Markdown",
    "Chat",
    "Code",
    "Terminal",
    "Flow",
    "Effects",
    "Graph",
];

struct Gallery {
    shell: ShellState,
    dark: bool,
    palette: Option<DialogResult<usize>>,
    shot_frame: u64,
    forms: sections::forms::FormsState,
    pickers: sections::pickers::PickersState,
    structure: sections::structure::StructureState,
    feedback: sections::feedback::FeedbackState,
    overlays: sections::overlays::OverlaysState,
    term: sections::term::TermSectionState,
    data: sections::data::DataState,
    files: sections::files::FilesState,
    board: sections::board::BoardState,
    date: sections::date::DateState,
    chat: sections::chat::ChatSectionState,
    code: sections::code::CodeSectionState,
    effects: sections::effects::EffectsState,
    graph: sections::graph::GraphState,
}

impl Gallery {
    fn new() -> Gallery {
        Gallery {
            shell: ShellState::default(),
            dark: true,
            palette: None,
            shot_frame: 0,
            forms: sections::forms::FormsState::default(),
            pickers: sections::pickers::PickersState::default(),
            structure: sections::structure::StructureState::default(),
            feedback: sections::feedback::FeedbackState::default(),
            overlays: sections::overlays::OverlaysState::default(),
            term: sections::term::TermSectionState::default(),
            data: sections::data::DataState::default(),
            files: sections::files::FilesState::default(),
            board: sections::board::BoardState::default(),
            date: sections::date::DateState::default(),
            chat: sections::chat::ChatSectionState::default(),
            code: sections::code::CodeSectionState::default(),
            effects: sections::effects::EffectsState::default(),
            graph: sections::graph::GraphState::default(),
        }
    }

    fn section_ui(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx, selected: usize) {
        match selected {
            0 => sections::primitives::draw(ui),
            1 => sections::feedback::draw(ui, ctx, &mut self.feedback),
            2 => sections::forms::draw(ui, &mut self.forms),
            3 => sections::pickers::draw(ui, &mut self.pickers),
            4 => sections::structure::draw(ui, &mut self.structure),
            5 => sections::overlays::draw(ui, ctx, &mut self.overlays),
            6 => sections::data::draw(ui, &mut self.data),
            7 => sections::files::draw(ui, &mut self.files),
            8 => sections::board::draw(ui, &mut self.board),
            9 => sections::charts::draw(ui),
            10 => sections::date::draw(ui, &mut self.date),
            11 => sections::text::draw(ui),
            12 => sections::chat::draw(ui, &mut self.chat),
            13 => sections::code::draw(ui, &mut self.code),
            14 => sections::term::draw(ui, &mut self.term),
            15 => sections::flow::draw(ui),
            16 => sections::effects::draw(ui, ctx, &mut self.effects),
            17 => sections::graph::draw(ui, &mut self.graph),
            _ => {
                let _ = forge_egui::widgets::Empty::new(SECTIONS[selected])
                    .message("Lands in a later milestone")
                    .show(ui);
            }
        }
    }
}

impl App for Gallery {
    fn tick(&mut self, _dt: f32, ctx: &mut Ctx) {
        // Poll the palette's DialogResult cell — resolves frames later.
        if let Some(result) = &self.palette {
            if let Some(index) = result.take() {
                self.shell.selected = index;
                self.palette = None;
            }
        }
        self_shot::tick(ctx.egui(), &mut self.shot_frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut Ctx) {
        if !ctx.dialog_open()
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::K))
        {
            let commands = SECTIONS
                .iter()
                .map(|s| Command::new(*s).hint("section"))
                .collect();
            self.palette = Some(ctx.open_palette(commands));
        }

        let dark = self.dark;
        let sections = [
            NavSection::new(Some("Basics"), &SECTIONS[0..2]),
            NavSection::new(Some("Forms"), &SECTIONS[2..4]),
            NavSection::new(Some("Structure"), &SECTIONS[4..6]),
            NavSection::new(Some("Data"), &SECTIONS[6..9]),
            NavSection::new(Some("Viz"), &SECTIONS[9..11]),
            NavSection::new(Some("Specialty"), &SECTIONS[11..18]),
        ];
        let mut toggle_theme = false;
        let shell = Shell::new("◆ FORGE", &sections)
            .subtitle("egui gallery")
            .topbar(SECTIONS[self.shell.selected])
            .topbar_right(|ui| {
                if Button::new(if dark { "Light" } else { "Dark" })
                    .small(true)
                    .variant(Variant::Ghost)
                    .show(ui)
                    .clicked()
                {
                    toggle_theme = true;
                }
            })
            .status("Ctrl+B sidebar · Ctrl+K palette")
            .status_right("forge-egui 0.1");

        let mut shell_state = std::mem::take(&mut self.shell);
        let selected = shell_state.selected;
        let _ = shell.show(ui, &mut shell_state, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.section_ui(ui, ctx, selected);
            });
        });
        self.shell = shell_state;

        if toggle_theme {
            self.dark = !self.dark;
            ctx.set_theme(if self.dark {
                Theme::dark()
            } else {
                Theme::light()
            });
        }
    }
}

/// Headless-friendly self-capture for development: set `FORGE_GALLERY_SHOT`
/// to a PNG path (and optionally `FORGE_GALLERY_SECTION` to a section index)
/// and the gallery screenshots itself after a few frames, saves, and exits —
/// no window-manager focus games.
mod self_shot {
    use forge_egui::egui;

    pub fn tick(ctx: &egui::Context, frame: &mut u64) {
        let Ok(path) = std::env::var("FORGE_GALLERY_SHOT") else {
            return;
        };
        *frame += 1;
        ctx.request_repaint();
        if *frame == 20 {
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
    let mut gallery = Gallery::new();
    if let Ok(section) = std::env::var("FORGE_GALLERY_SECTION") {
        gallery.shell.selected = section.parse().unwrap_or(0);
    }
    forge_egui::run(
        gallery,
        Theme::dark(),
        RunOptions {
            window_title: "Forge — egui gallery".to_owned(),
            ..Default::default()
        },
    )
}
