//! Particle FX: explode / recreate / materialize / sparkle over a demo card.

use forge_egui::prelude::*;

#[derive(Default)]
pub struct EffectsState {
    hidden: bool,
}

pub fn draw(ui: &mut egui::Ui, ctx: &mut Ctx, state: &mut EffectsState) {
    let t = Theme::of(ui.ctx());

    Card::new().title("Particle FX").show(ui, |ui| {
        ui.label(
            egui::RichText::new(
                "Effects run on the runtime's foreground layer. Motion preference comes from \
                 RunOptions::motion / FORGE_EGUI_MOTION (full · reduced · off).",
            )
            .size(t.type_scale.sm)
            .color(t.fg[2]),
        );
        ui.add_space(8.0);

        // The victim card the effects target.
        let target = if state.hidden {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(320.0, 96.0), egui::Sense::hover());
            if !ctx.fx_active_in(rect) {
                state.hidden = false; // respawn once the dust settles
            }
            rect
        } else {
            Card::new()
                .title("Test subject")
                .show(ui, |ui| {
                    ui.set_max_width(288.0);
                    ui.label(egui::RichText::new("Aim the effects at me.").color(t.fg[1]));
                    let _ = Badge::new("stable")
                        .tone(forge_egui::widgets::Tone::Success)
                        .show(ui);
                })
                .response
                .rect
        };
        ui.add_space(12.0);

        ui.horizontal_wrapped(|ui| {
            if Button::new("Explode")
                .variant(Variant::Danger)
                .show(ui)
                .clicked()
            {
                ctx.fx().explode(target);
                state.hidden = true;
            }
            if Button::new("Recreate").show(ui).clicked() {
                ctx.fx().recreate(target);
            }
            if Button::new("Materialize").show(ui).clicked() {
                ctx.fx().materialize(target);
            }
            if Button::new("Sparkle")
                .variant(Variant::Primary)
                .show(ui)
                .clicked()
            {
                ctx.fx().sparkle(target);
            }
            if Button::new("Brand sparkle")
                .variant(Variant::Ghost)
                .show(ui)
                .clicked()
            {
                ctx.fx().sparkle_with(
                    target,
                    vec![t.warning.base, t.warning.fg, t.accent.contrast],
                );
            }
        });
    });
}
