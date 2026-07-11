//! Primitives: buttons, badges, avatars, stats, and the small display atoms.

use forge_egui::prelude::*;
use forge_egui::widgets::Tone;

pub fn draw(ui: &mut egui::Ui) {
    let t = Theme::of(ui.ctx());

    Card::new().title("Buttons").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let _ = Button::new("Default").show(ui);
            let _ = Button::new("Primary").variant(Variant::Primary).show(ui);
            let _ = Button::new("Ghost").variant(Variant::Ghost).show(ui);
            let _ = Button::new("Danger").variant(Variant::Danger).show(ui);
            let _ = Button::new("Disabled").disabled(true).show(ui);
            let _ = Button::new("Small").small(true).show(ui);
            let _ = Button::new("With icon").icon(Glyph::Plus).show(ui);
            let _ = IconButton::new(Glyph::Gear, "Settings").show(ui);
            let _ = IconButton::new(Glyph::Cross, "Close")
                .variant(Variant::Default)
                .show(ui);
        });
    });
    ui.add_space(12.0);

    Card::new().title("Badges").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, tone) in [
                ("neutral", Tone::Neutral),
                ("accent", Tone::Accent),
                ("success", Tone::Success),
                ("warning", Tone::Warning),
                ("danger", Tone::Danger),
                ("info", Tone::Info),
            ] {
                let _ = Badge::new(label).tone(tone).show(ui);
            }
            let _ = Badge::new("with dot")
                .tone(Tone::Success)
                .dot(true)
                .show(ui);
        });
    });
    ui.add_space(12.0);

    Grid::new(3).show(ui, |g| {
        g.cell(|ui| {
            let _ = Stat::new("Requests / s", "12,408")
                .delta("4.3% vs last hour", Trend::Up, Tone::Success)
                .show(ui);
        });
        g.cell(|ui| {
            let _ = Stat::new("P99 latency", "231 ms")
                .delta("12 ms worse", Trend::Down, Tone::Danger)
                .show(ui);
        });
        g.cell(|ui| {
            let _ = Stat::new("Open incidents", "3").show(ui);
        });
    });

    Card::new().title("Avatars & status").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let _ = Avatar::new("Wil Taylor").size(AvatarSize::Lg).show(ui);
            let _ = Avatar::new("Ada Lovelace").status(Tone::Success).show(ui);
            let _ = Avatar::new("Grace Hopper").size(AvatarSize::Sm).show(ui);
            let _ = Separator::new().vertical().show(ui);
            for tone in [Tone::Success, Tone::Warning, Tone::Danger, Tone::Info] {
                let _ = StatusDot::new(tone).show(ui);
            }
            let _ = StatusDot::new(Tone::Success).pulse(true).show(ui);
            let _ = Separator::new().vertical().show(ui);
            let _ = Kbd::new("Ctrl").show(ui);
            let _ = Kbd::new("K").show(ui);
        });
    });
    ui.add_space(12.0);

    Grid::new(2).show(ui, |g| {
        g.cell(|ui| {
            Card::new().title("Skeleton").show(ui, |ui| {
                let _ = Skeleton::new().show(ui);
                let _ = Skeleton::new().width(180.0).show(ui);
                let _ = Skeleton::new().width(120.0).height(24.0).show(ui);
            });
        });
        g.cell(|ui| {
            Card::new()
                .title("Empty state")
                .padded(false)
                .show(ui, |ui| {
                    let _ = Empty::new("No results")
                        .message("Try a different search term")
                        .icon(Glyph::Search)
                        .show_with(ui, |ui| {
                            let _ = Button::new("Clear filters").small(true).show(ui);
                        });
                });
        });
    });

    Card::new().title("Typography").show(ui, |ui| {
        Eyebrow::new("Eyebrow caption").show(ui);
        ui.label(
            egui::RichText::new("Heading — The quick brown fox")
                .font(t.font(
                    ui.ctx(),
                    forge_egui::theme::FontWeight::SemiBold,
                    t.type_scale.h3,
                ))
                .color(t.fg[0]),
        );
        ui.label(egui::RichText::new("Body — jumps over the lazy dog.").color(t.fg[1]));
        ui.label(
            egui::RichText::new("mono — cargo run -p egui-gallery")
                .font(t.mono(t.type_scale.base))
                .color(t.fg[2]),
        );
    });
}
