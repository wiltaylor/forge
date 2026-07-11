//! Action invocation: dispatch registered actions with a JSON payload,
//! exactly as a Forge server's `POST /api/actions/{name}` would.

use crate::backend::{Backend, Job};
use forge_egui::prelude::*;
use serde_json::Value;

pub struct ActionsPage {
    selected: usize,
    payload: String,
    output: Option<String>,
    job: Option<Job<Value>>,
}

impl Default for ActionsPage {
    fn default() -> Self {
        ActionsPage {
            selected: 0,
            payload: "{\n  \"hello\": \"forge\"\n}".to_owned(),
            output: None,
            job: None,
        }
    }
}

const ACTIONS: &[&str] = &["echo", "system_info", "publish"];

impl ActionsPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, backend: &Backend) {
        if let Some(result) = self.job.as_ref().and_then(Job::poll) {
            self.job = None;
            self.output = Some(match result {
                Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            });
        }
        let t = Theme::of(ui.ctx());

        Card::new().title("Invoke an action").show(ui, |ui| {
            let _ = forge_egui::widgets::ToggleGroup::new(&mut self.selected, ACTIONS).show(ui);
            ui.add_space(8.0);
            let json_error = serde_json::from_str::<Value>(&self.payload)
                .err()
                .map(|e| e.to_string());
            let mut area = forge_egui::widgets::Textarea::new(&mut self.payload)
                .label("Payload (JSON)")
                .rows(5);
            if let Some(err) = json_error.as_deref() {
                area = area.error(err);
            }
            let _ = area.show(ui);
            let running = self.job.is_some();
            if Button::new(if running { "Running…" } else { "Run" })
                .variant(Variant::Primary)
                .disabled(running || json_error.is_some())
                .show(ui)
                .clicked()
            {
                let payload: Value = serde_json::from_str(&self.payload).unwrap_or(Value::Null);
                self.job = Some(backend.invoke(ui.ctx(), ACTIONS[self.selected], payload));
                self.output = None;
            }
        });
        ui.add_space(12.0);

        if let Some(output) = &self.output {
            Card::new().title("Result").show(ui, |ui| {
                ui.label(
                    egui::RichText::new(output)
                        .font(t.mono(t.type_scale.base))
                        .color(t.fg[1]),
                );
            });
        }
    }
}
