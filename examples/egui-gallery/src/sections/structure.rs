//! Navigation & structure: crumbs, page head, tabs, pagination, settings
//! rows, split panes.

use forge_egui::prelude::*;
use forge_egui::widgets::{
    Crumbs, PageHead, Pagination, SettingsRow, SettingsSection, SplitPane, SplitState, TabItem,
    Tabs,
};

pub struct StructureState {
    tab: usize,
    page: usize,
    split: SplitState,
    notifications: bool,
    compact: bool,
}

impl Default for StructureState {
    fn default() -> Self {
        StructureState {
            tab: 0,
            page: 4,
            split: SplitState::default(),
            notifications: true,
            compact: false,
        }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut StructureState) {
    let t = Theme::of(ui.ctx());

    Card::new().title("Page head & crumbs").show(ui, |ui| {
        let _ = Crumbs::new(&["forge", "gallery", "structure"]).show(ui);
        ui.add_space(8.0);
        let _ = PageHead::new("Deploy pipeline")
            .eyebrow("Operations")
            .sub("Every push to main builds, tests, and ships")
            .actions(|ui| {
                let _ = Button::new("New deploy").variant(Variant::Primary).show(ui);
                let _ = Button::new("Settings").variant(Variant::Ghost).show(ui);
            })
            .show(ui);
    });
    ui.add_space(12.0);

    Card::new().title("Tabs & pagination").show(ui, |ui| {
        let items = [
            TabItem::new("Overview"),
            TabItem::new("Deploys").count(12),
            TabItem::new("Logs"),
            TabItem::new("Locked").disabled(true),
        ];
        let _ = Tabs::new(&mut state.tab, &items).show(ui);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("active tab: {}", state.tab))
                .font(t.mono(t.type_scale.sm))
                .color(t.fg[2]),
        );
        ui.add_space(12.0);
        let _ = Pagination::new(&mut state.page, 20).show(ui);
    });
    ui.add_space(12.0);

    Card::new()
        .title("Split pane")
        .padded(false)
        .show(ui, |ui| {
            let _ = SplitPane::new(&mut state.split)
                .height(180.0)
                .min(120.0)
                .show(
                    ui,
                    |ui| {
                        ui.label(egui::RichText::new("Navigator").color(ui.visuals().text_color()));
                        ui.label(egui::RichText::new("drag the divider →").size(12.0));
                    },
                    |ui| {
                        ui.label("Detail pane");
                    },
                );
        });
    ui.add_space(12.0);

    let _ = SettingsSection::new("Notifications")
        .sub("What lands in your inbox")
        .show(ui, |ui| {
            let _ = SettingsRow::new("Email digests")
                .help("A summary every Monday morning")
                .show(ui, |ui| {
                    let _ = forge_egui::widgets::Toggle::new(&mut state.notifications).show(ui);
                });
            let _ = SettingsRow::new("Compact density")
                .help("Tighter rows in tables and lists")
                .show(ui, |ui| {
                    let _ = forge_egui::widgets::Toggle::new(&mut state.compact).show(ui);
                });
        });
}
