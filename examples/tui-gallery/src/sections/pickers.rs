use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const NOTES: FocusId = FocusId::new("pk-notes");
const REGION: FocusId = FocusId::new("pk-region");
const PKGS: FocusId = FocusId::new("pk-pkgs");
const CPU: FocusId = FocusId::new("pk-cpu");
const VIEW: FocusId = FocusId::new("pk-view");
const IMAGE: FocusId = FocusId::new("pk-image");

pub const REGIONS: &[&str] = &[
    "eu-west-1",
    "eu-central-1",
    "us-east-1",
    "us-west-2",
    "ap-southeast-2",
];
pub const PACKAGES: &[&str] = &[
    "forge-core",
    "forge-server",
    "forge-auth",
    "forge-tui",
    "forge-tauri",
    "gallery",
    "parity",
];
pub const VIEWS: &[&str] = &["grid", "list", "table"];
pub const IMAGES: &[&str] = &[
    "nixos/24.11-minimal",
    "debian/12-slim",
    "archlinux/rolling",
    "ubuntu/24.04",
    "alpine/3.20",
    "fedora/41",
];

pub struct PickersState {
    pub notes: TextareaState,
    pub region: SelectState,
    pub packages: ListBoxState,
    pub cpu: SliderState,
    pub view: ToggleGroupState,
    pub image: ComboboxState,
}

impl Default for PickersState {
    fn default() -> PickersState {
        let mut packages = ListBoxState::multi();
        packages.toggle(0);
        packages.toggle(3);
        PickersState {
            notes: TextareaState::with_value(
                "Deploy notes:\n- rolling restart\n- watch the event bus",
            ),
            region: SelectState::with_value(0),
            packages,
            cpu: SliderState::new(4.0, 1.0, 16.0, 1.0),
            view: ToggleGroupState::new(0),
            image: ComboboxState::new(),
        }
    }
}

impl PickersState {
    pub fn handle_key(
        &mut self,
        focused: Option<FocusId>,
        key: KeyEvent,
        ctx: &mut Ctx,
    ) -> Outcome {
        let outcome = match focused {
            Some(id) if id == NOTES => self.notes.handle_key(key),
            Some(id) if id == REGION => self.region.handle_key(key),
            Some(id) if id == PKGS => self.packages.handle_key(key),
            Some(id) if id == CPU => self.cpu.handle_key(key),
            Some(id) if id == VIEW => self.view.handle_key(key),
            Some(id) if id == IMAGE => self.image.handle_key(key, IMAGES),
            _ => Outcome::Ignored,
        };
        if outcome == Outcome::Submitted {
            if focused == Some(IMAGE) {
                ctx.toast()
                    .success(format!("Image: {}", self.image.input.value()));
            } else if focused == Some(REGION) {
                if let Some(i) = self.region.value {
                    ctx.toast().info(format!("Region: {}", REGIONS[i]));
                }
            }
            return Outcome::Consumed;
        }
        outcome
    }

    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        // Dropdown-carrying widgets first (their popups overlay the others).
        let out = self.region.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(REGION);
            if out == Outcome::Changed {
                if let Some(i) = self.region.value {
                    ctx.toast().info(format!("Region: {}", REGIONS[i]));
                }
            }
            return out;
        }
        let out = self.image.handle_mouse(ev, IMAGES);
        if out.is_handled() {
            ctx.focus.focus(IMAGE);
            if out == Outcome::Submitted {
                ctx.toast()
                    .success(format!("Image: {}", self.image.input.value()));
            }
            return out;
        }
        macro_rules! try_widget {
            ($state:expr, $id:expr) => {
                let out = $state.handle_mouse(ev);
                if out.is_handled() {
                    ctx.focus.focus($id);
                    return out;
                }
            };
        }
        try_widget!(self.cpu, CPU);
        try_widget!(self.view, VIEW);
        try_widget!(self.packages, PKGS);
        try_widget!(self.notes, NOTES);
        Outcome::Ignored
    }

    pub fn paste(&mut self, focused: Option<FocusId>, text: &str) {
        match focused {
            Some(id) if id == NOTES => self.notes.insert_str(text),
            Some(id) if id == IMAGE => self.image.insert_str(text, IMAGES),
            _ => {}
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut PickersState) {
    // Register focus in VISUAL order (Tab order = registration order), even
    // though the dropdown-carrying widgets render last for z-order.
    let f_region = ctx.focus.register(REGION);
    let f_image = ctx.focus.register(IMAGE);
    let f_cpu = ctx.focus.register(CPU);
    let f_view = ctx.focus.register(VIEW);
    let f_pkgs = ctx.focus.register(PKGS);
    let f_notes = ctx.focus.register(NOTES);

    let cols = Grid::new(2).gap(3).cells(area, 2, area.height);
    let left = cols[0];
    let right = cols[1];

    // Left column rects (top to bottom).
    let mut y = left.y;
    let bottom = left.y + left.height;
    let row = |h: u16, gap: u16, y: &mut u16| -> Option<Rect> {
        if *y + h > bottom {
            return None;
        }
        let r = Rect::new(left.x, *y, left.width.min(40), h);
        *y += h + gap;
        Some(r)
    };

    let lbl_region = row(1, 0, &mut y);
    let r_region = row(1, 1, &mut y);
    let lbl_image = row(1, 0, &mut y);
    let r_image = row(1, 1, &mut y);
    let lbl_cpu = row(1, 0, &mut y);
    let r_cpu = row(1, 1, &mut y);
    let lbl_view = row(1, 0, &mut y);
    let r_view = row(1, 1, &mut y);
    let lbl_pkgs = row(1, 0, &mut y);
    let r_pkgs = row(5, 0, &mut y);

    for (r, s) in [
        (lbl_region, "Region (Select)"),
        (lbl_image, "Image (Combobox)"),
        (lbl_cpu, "CPU cores (Slider)"),
        (lbl_view, "View (ToggleGroup)"),
        (lbl_pkgs, "Packages (ListBox multi)"),
    ] {
        if let Some(r) = r {
            frame.render_widget(Eyebrow::new(s).theme(t), r);
        }
    }

    if let Some(r) = r_cpu {
        frame.render_stateful_widget(Slider::new().focused(f_cpu).theme(t), r, &mut state.cpu);
    }
    if let Some(r) = r_view {
        frame.render_stateful_widget(
            ToggleGroup::new(VIEWS).focused(f_view).theme(t),
            r,
            &mut state.view,
        );
    }
    if let Some(r) = r_pkgs {
        frame.render_stateful_widget(
            ListBox::new(PACKAGES).focused(f_pkgs).theme(t),
            r,
            &mut state.packages,
        );
    }

    // Right column: textarea.
    if right.height > 2 {
        frame.render_widget(
            Eyebrow::new("Notes (Textarea)").theme(t),
            Rect::new(right.x, right.y, right.width, 1),
        );
        frame.render_stateful_widget(
            Textarea::new()
                .placeholder("Notes…")
                .focused(f_notes)
                .theme(t),
            Rect::new(
                right.x,
                right.y + 1,
                right.width.min(44),
                right.height.saturating_sub(1).min(8),
            ),
            &mut state.notes,
        );
    }

    // Dropdown-carrying widgets render LAST so their popups overpaint.
    if let Some(r) = r_region {
        frame.render_stateful_widget(
            Select::new(REGIONS).focused(f_region).theme(t),
            r,
            &mut state.region,
        );
    }
    if let Some(r) = r_image {
        frame.render_stateful_widget(
            Combobox::new(IMAGES)
                .placeholder("Search images…")
                .focused(f_image)
                .theme(t),
            r,
            &mut state.image,
        );
    }
}
