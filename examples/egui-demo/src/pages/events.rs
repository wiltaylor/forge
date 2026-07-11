//! Live event feed off the in-process bus: the 2s ticker, anything published
//! from the Actions page, and a publish row — with visible lag handling.

use crate::backend::{Backend, EventFeed};
use forge_egui::prelude::*;
use forge_egui::widgets::Tone;

pub struct EventsPage {
    filter: String,
    topic: String,
    payload: String,
}

impl Default for EventsPage {
    fn default() -> Self {
        EventsPage {
            filter: String::new(),
            topic: "demo".to_owned(),
            payload: "{\"msg\":\"hi\"}".to_owned(),
        }
    }
}

impl EventsPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, backend: &Backend, feed: Option<&EventFeed>) {
        let t = Theme::of(ui.ctx());

        Card::new().title("Publish").show(ui, |ui| {
            ui.horizontal(|ui| {
                let _ = forge_egui::widgets::Input::new(&mut self.topic)
                    .placeholder("topic")
                    .desired_width(140.0)
                    .show(ui);
                let _ = forge_egui::widgets::Input::new(&mut self.payload)
                    .placeholder("{\"json\":true}")
                    .desired_width(280.0)
                    .show(ui);
                if Button::new("Publish").small(true).show(ui).clicked() {
                    let data: serde_json::Value =
                        serde_json::from_str(&self.payload).unwrap_or(serde_json::Value::Null);
                    backend.bus.publish(&self.topic, data);
                }
            });
        });
        ui.add_space(12.0);

        Card::new().title("Live feed").show(ui, |ui| {
            ui.horizontal(|ui| {
                let _ = forge_egui::widgets::Input::new(&mut self.filter)
                    .placeholder("filter by topic…")
                    .icon(Glyph::Search)
                    .desired_width(220.0)
                    .show(ui);
                let _ = Badge::new("ticks every 2s").tone(Tone::Info).show(ui);
                if let Some(feed) = feed {
                    if feed.lagged > 0 {
                        let _ = Badge::new(&format!("lagged ×{}", feed.lagged))
                            .tone(Tone::Warning)
                            .show(ui);
                    }
                }
            });
            ui.add_space(6.0);

            let Some(feed) = feed else {
                let _ = Empty::new("Feed starting…").show(ui);
                return;
            };
            let filter = self.filter.to_lowercase();
            egui::ScrollArea::vertical()
                .max_height(420.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for entry in &feed.entries {
                        if !filter.is_empty() && !entry.topic.to_lowercase().contains(&filter) {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            let tone = if entry.topic == "(lagged)" {
                                Tone::Warning
                            } else {
                                Tone::Accent
                            };
                            let _ = Badge::new(&entry.topic).tone(tone).show(ui);
                            ui.label(
                                egui::RichText::new(&entry.json)
                                    .font(t.mono(t.type_scale.sm))
                                    .color(t.fg[1]),
                            );
                        });
                    }
                });
        });
    }
}
