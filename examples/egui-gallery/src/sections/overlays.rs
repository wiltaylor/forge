//! Overlays: modal, sheet, popover, tooltip, menus, plus the runtime dialogs
//! (confirm / danger confirm / palette) with their polled results shown as
//! toasts.

use forge_egui::prelude::*;

#[derive(Default)]
pub struct OverlaysState {
    modal_open: bool,
    modal_xl_open: bool,
    sheet_right_open: bool,
    sheet_left_open: bool,
    modal_name: String,
    confirm: Option<DialogResult<bool>>,
    palette: Option<DialogResult<usize>>,
}

pub fn draw(ui: &mut egui::Ui, ctx: &mut Ctx, state: &mut OverlaysState) {
    // Poll runtime dialog results resolved in earlier frames.
    if let Some(result) = &state.confirm {
        if let Some(confirmed) = result.take() {
            if confirmed {
                ctx.toast().success("Confirmed");
            } else {
                ctx.toast().info("Cancelled");
            }
            state.confirm = None;
        }
    }
    if let Some(result) = &state.palette {
        if let Some(index) = result.take() {
            ctx.toast().info(format!("Palette picked #{index}"));
            state.palette = None;
        }
    }

    Card::new().title("Modal").show(ui, |ui| {
        ui.horizontal(|ui| {
            if Button::new("Open modal (md)").show(ui).clicked() {
                state.modal_open = true;
            }
            if Button::new("Open modal (xl, footer)").show(ui).clicked() {
                state.modal_xl_open = true;
            }
        });
    });
    ui.add_space(12.0);

    Card::new().title("Sheet").show(ui, |ui| {
        ui.horizontal(|ui| {
            if Button::new("Open right sheet").show(ui).clicked() {
                state.sheet_right_open = true;
            }
            if Button::new("Open left sheet").show(ui).clicked() {
                state.sheet_left_open = true;
            }
        });
    });
    ui.add_space(12.0);

    Card::new().title("Popover & tooltip").show(ui, |ui| {
        ui.horizontal(|ui| {
            let _ = Popover::new("gallery-popover").width(260.0).show(
                ui,
                |ui| Button::new("Toggle popover").show(ui),
                |ui| {
                    ui.label("Anchored floating panel.");
                    ui.add_space(6.0);
                    let _ = Progress::new(0.4).label("Inline widget").show(ui);
                },
            );
            let r = Button::new("Hover me").variant(Variant::Ghost).show(ui);
            let _ = tooltip(r.response, "Forge-styled tooltip");
            let r = Button::new("Me too").variant(Variant::Ghost).show(ui);
            let _ = tooltip(r.response, "Small, dense, bg[4]");
        });
    });
    ui.add_space(12.0);

    Card::new().title("Menus").show(ui, |ui| {
        let items = [
            MenuItem::new("Copy").icon(Glyph::File),
            MenuItem::new("Duplicate").icon(Glyph::Plus),
            MenuItem::new("Locked").disabled(true),
            MenuItem::new("Delete")
                .icon(Glyph::Cross)
                .danger(true)
                .separator_before(true),
        ];
        ui.horizontal(|ui| {
            if let Some(index) =
                DropdownMenu::new(&items).show(ui, |ui| Button::new("Dropdown ▾").show(ui))
            {
                ctx.toast().info(format!("Menu: {}", items[index].label));
            }

            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(220.0, 32.0), egui::Sense::click());
            let t = ctx.theme().clone();
            ui.painter().rect_filled(rect, t.radius.md, t.bg[2]);
            ui.painter().rect_stroke(
                rect,
                t.radius.md,
                egui::Stroke::new(1.0, t.border.default),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Right-click zone",
                t.mono(t.type_scale.sm),
                t.fg[2],
            );
            if let Some(index) = context_menu(&response, &items) {
                ctx.toast().info(format!("Context: {}", items[index].label));
            }
        });
    });
    ui.add_space(12.0);

    Card::new().title("Runtime dialogs").show(ui, |ui| {
        ui.horizontal(|ui| {
            if Button::new("Confirm").show(ui).clicked() {
                state.confirm =
                    Some(ctx.confirm("Apply changes?", "Rolls the config to all nodes."));
            }
            if Button::new("Confirm danger")
                .variant(Variant::Danger)
                .show(ui)
                .clicked()
            {
                state.confirm =
                    Some(ctx.confirm_danger("Delete replica?", "This cannot be undone.", "Delete"));
            }
            if Button::new("Command palette").show(ui).clicked() {
                state.palette = Some(ctx.open_palette(vec![
                    Command::new("Restart agent").hint("service"),
                    Command::new("Tail logs").hint("logs"),
                    Command::new("Rotate keys").hint("security"),
                ]));
            }
        });
    });

    // The overlays themselves.
    let _ = Modal::new("Modal title").show(ui.ctx(), &mut state.modal_open, |ui| {
        ui.label("Body content — Esc, scrim, or × closes.");
        ui.add_space(8.0);
        let _ = Input::new(&mut state.modal_name)
            .label("Name")
            .placeholder("Focus works inside")
            .show(ui);
    });

    let mut xl_close = false;
    let _ = Modal::new("Wide modal")
        .width(ModalWidth::Xl)
        .footer(|ui| {
            if Button::new("Save")
                .variant(Variant::Primary)
                .show(ui)
                .clicked()
            {
                xl_close = true;
            }
            let _ = Button::new("Cancel").variant(Variant::Ghost).show(ui);
        })
        .show(ui.ctx(), &mut state.modal_xl_open, |ui| {
            let _ = Alert::new(Tone::Info, "960pt wide")
                .message("The footer sits below a subtle separator.")
                .show(ui);
        });
    if xl_close {
        state.modal_xl_open = false;
    }

    let _ = Sheet::new("Right sheet").show(ui, &mut state.sheet_right_open, |ui| {
        ui.label("Slides in over the content.");
        ui.add_space(8.0);
        let _ = Progress::new(0.8)
            .label("Detail view")
            .show_value(true)
            .show(ui);
    });
    let _ = Sheet::new("Left sheet").side(Side::Left).width(300.0).show(
        ui,
        &mut state.sheet_left_open,
        |ui| {
            ui.label("Left edge variant.");
        },
    );
}
