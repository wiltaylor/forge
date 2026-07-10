use forge_tui::event::{KeyCombo, Keymap};
use forge_tui::prelude::*;
use forge_tui::runtime::dialogs::OwnedMenuEntry;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const BTN: &str = "ov-btn";
const BUTTONS: [&str; 5] = ["Confirm…", "Delete…", "Menu", "Palette", "Help"];

#[derive(Default)]
pub struct OverlaysState {
    confirm: Option<DialogResult<bool>>,
    delete: Option<DialogResult<bool>>,
    menu: Option<DialogResult<Option<usize>>>,
    palette: Option<DialogResult<Option<String>>>,
    menu_anchor: Rect,
    btn_rects: Vec<Rect>,
}

const MENU_LABELS: [&str; 4] = ["Restart", "Drain", "Cordon", "Delete node"];

impl OverlaysState {
    pub fn open_palette(&mut self, ctx: &mut Ctx) {
        let commands = vec![
            OwnedCommand::new("toast-info", "Show info toast"),
            OwnedCommand::new("toast-error", "Show error toast"),
            OwnedCommand::new("deploy", "Deploy to production").kbd("⌃D"),
            OwnedCommand::new("restart", "Restart event bus"),
            OwnedCommand::new("logs", "Open logs"),
            OwnedCommand::new("theme", "Toggle theme").kbd("T"),
        ];
        let (overlay, result) = PaletteOverlay::new(commands);
        self.palette = Some(result);
        ctx.open(Box::new(overlay));
    }

    pub fn open_help(&mut self, ctx: &mut Ctx) {
        let keymap = Keymap::new()
            .bind("palette", KeyCombo::ctrl(KeyCode::Char('k')), "Open the command palette")
            .bind("help", KeyCombo::char('?'), "This help")
            .bind("theme", KeyCombo::char('t'), "Toggle dark/light")
            .bind("nav", KeyCombo::new(KeyCode::Tab), "Move focus")
            .bind("quit", KeyCombo::char('q'), "Quit the gallery");
        ctx.open(Box::new(HelpOverlay::new(&keymap)));
    }

    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        for (i, rect) in self.btn_rects.clone().into_iter().enumerate() {
            if forge_tui::event::clicked(ev, rect) {
                ctx.focus.focus(FocusId::indexed(BTN, i as u32));
                self.activate(i, ctx);
                return Outcome::Consumed;
            }
        }
        Outcome::Ignored
    }

    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        if key.code == KeyCode::Char('s') {
            ctx.open(Box::new(SheetDemo));
            return Outcome::Consumed;
        }
        if !matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
            return Outcome::Ignored;
        }
        for i in 0..BUTTONS.len() {
            if focused != Some(FocusId::indexed(BTN, i as u32)) {
                continue;
            }
            self.activate(i, ctx);
            return Outcome::Consumed;
        }
        Outcome::Ignored
    }

    fn activate(&mut self, i: usize, ctx: &mut Ctx) {
        match i {
            0 => {
                let (dialog, result) =
                    ConfirmDialog::new("Deploy build?", "Rolls out gallery v0.1 to all nodes.");
                self.confirm = Some(result);
                ctx.open(Box::new(dialog));
            }
            1 => {
                let (dialog, result) = ConfirmDialog::new(
                    "Delete node-3?",
                    "The node drains first; this cannot be undone.",
                );
                let dialog = dialog.confirm_label("Delete").danger();
                self.delete = Some(result);
                ctx.open(Box::new(dialog));
            }
            2 => {
                let entries = vec![
                    OwnedMenuEntry::section("Node"),
                    OwnedMenuEntry::item_kbd(MENU_LABELS[0], "R"),
                    OwnedMenuEntry::item(MENU_LABELS[1]),
                    OwnedMenuEntry::item(MENU_LABELS[2]),
                    OwnedMenuEntry::separator(),
                    OwnedMenuEntry::danger(MENU_LABELS[3]),
                ];
                let (overlay, result) = MenuOverlay::new(entries, self.menu_anchor);
                self.menu = Some(result);
                ctx.open(Box::new(overlay));
            }
            3 => self.open_palette(ctx),
            _ => self.open_help(ctx),
        }
    }

    /// Poll dialog results (runs on the runtime tick).
    pub fn poll_results(&mut self, ctx: &mut Ctx) {
        if let Some(v) = self.confirm.as_ref().and_then(|r| r.take()) {
            if v {
                ctx.toast().success("Deploy confirmed");
            } else {
                ctx.toast().info("Deploy cancelled");
            }
            self.confirm = None;
        }
        if let Some(v) = self.delete.as_ref().and_then(|r| r.take()) {
            if v {
                ctx.toast().error("node-3 deleted");
            } else {
                ctx.toast().info("Deletion cancelled");
            }
            self.delete = None;
        }
        if let Some(v) = self.menu.as_ref().and_then(|r| r.take()) {
            match v {
                Some(i) => ctx.toast().info(format!("Menu: {}", MENU_LABELS[i.min(3)])),
                None => ctx.toast().info("Menu dismissed"),
            }
            self.menu = None;
        }
        if let Some(v) = self.palette.as_ref().and_then(|r| r.take()) {
            match v.as_deref() {
                Some("toast-error") => ctx.toast().error("Example error toast"),
                Some(id) => ctx.toast().info(format!("Palette: {id}")),
                None => {}
            }
            self.palette = None;
        }
    }
}

/// Custom overlay dogfooding the `Sheet` chrome widget.
struct SheetDemo;

impl Overlay for SheetDemo {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let sheet = Sheet::new(Side::Right).size(34).title(" node-3 ").theme(theme);
        let inner = sheet.inner(area);
        frame.render_widget(sheet, area);
        let rows = [
            ("state", "ready"),
            ("kernel", "6.12.9"),
            ("pods", "23"),
            ("cpu", "42%"),
            ("memory", "61%"),
        ];
        let buf = frame.buffer_mut();
        for (i, (k, v)) in rows.iter().enumerate() {
            let y = inner.y + 1 + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            buf.set_string(inner.x + 1, y, *k, ratatui::style::Style::new().fg(theme.fg[2]).bg(theme.bg[1]));
            buf.set_string(inner.x + 12, y, *v, ratatui::style::Style::new().fg(theme.fg[0]).bg(theme.bg[1]));
        }
    }

    fn handle(&mut self, _event: &ratatui::crossterm::event::Event) -> OverlayOutcome {
        OverlayOutcome::Ignored // Esc closes via the stack default
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut OverlaysState) {
    state.btn_rects.clear();
    let mut y = area.y;
    let bottom = area.y + area.height;
    let row = |h: u16, gap: u16, y: &mut u16| -> Option<Rect> {
        if *y + h > bottom {
            return None;
        }
        let r = Rect::new(area.x, *y, area.width, h);
        *y += h + gap;
        Some(r)
    };

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Dialogs · Menus · Palette").theme(t), r);
    }
    if let Some(r) = row(1, 1, &mut y) {
        let mut bx = r.x;
        for (i, label) in BUTTONS.iter().enumerate() {
            let focused = ctx.focus.register(FocusId::indexed(BTN, i as u32));
            let variant = match i {
                0 => Variant::Primary,
                1 => Variant::Danger,
                _ => Variant::Default,
            };
            let b = Button::new(label).variant(variant).focused(focused).theme(t);
            let bw = b.width();
            if bx + bw > r.x + r.width {
                break;
            }
            if i == 2 {
                state.menu_anchor = Rect::new(bx, r.y, bw, 1);
            }
            state.btn_rects.push(Rect::new(bx, r.y, bw, 1));
            frame.render_widget(b, Rect::new(bx, r.y, bw, 1));
            bx += bw + 2;
        }
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Eyebrow::new("Tooltip · Popover (inline)").theme(t), r);
    }
    if let Some(r) = row(8, 1, &mut y) {
        // Static anchored demos: a badge with a tooltip, a popover panel.
        let anchor = Rect::new(r.x + 2, r.y + 1, 10, 1);
        frame.render_widget(Badge::new("hover me").severity(Severity::Info).theme(t), anchor);
        frame.render_widget(Tooltip::new("Tooltips anchor to a rect", anchor).theme(t), r);

        let pop_anchor = Rect::new(r.x + 30, r.y, 12, 1);
        frame.render_widget(Badge::new("popover ▾").theme(t), pop_anchor);
        let popover = Popover::new(pop_anchor).size(30, 6).title(" Node info ").theme(t);
        let inner = popover.inner(r);
        frame.render_widget(popover, r);
        let buf = frame.buffer_mut();
        for (i, (k, v)) in [("state", "ready"), ("cpu", "42%"), ("uptime", "12d")].iter().enumerate() {
            let yy = inner.y + i as u16;
            if yy >= inner.y + inner.height {
                break;
            }
            buf.set_string(inner.x + 1, yy, *k, ratatui::style::Style::new().fg(t.fg[2]).bg(t.bg[4]));
            buf.set_string(inner.x + 10, yy, *v, ratatui::style::Style::new().fg(t.fg[0]).bg(t.bg[4]));
        }
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(
            Eyebrow::new("Sheet — press s (docked panel, Esc closes)").theme(t),
            r,
        );
    }
}
