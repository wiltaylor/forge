//! Doc-store scratchpad: list/get/put/delete against the on-disk JSON store,
//! with live name validation (`valid_doc_name`) and non-blocking `Job`s.

use crate::backend::{spawn_job, Backend, Job};
use forge_egui::prelude::*;
use serde_json::Value;

#[derive(Default)]
pub struct DocsPage {
    name: String,
    body: String,
    list: Vec<String>,
    listed_once: bool,
    status: Option<String>,
    list_job: Option<Job<Vec<Value>>>,
    get_job: Option<Job<Value>>,
    put_job: Option<Job<()>>,
    delete_job: Option<Job<()>>,
}

impl DocsPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, backend: &Backend, ctx: &mut Ctx) {
        self.poll(ctx);
        if !self.listed_once {
            self.listed_once = true;
            self.refresh(backend, ui.ctx());
        }

        let name_error = if self.name.is_empty() || forge_core::valid_doc_name(&self.name) {
            None
        } else {
            Some("lowercase alphanumeric, dash, underscore; max 64")
        };
        let json_error = if self.body.trim().is_empty() {
            None
        } else {
            serde_json::from_str::<Value>(&self.body)
                .err()
                .map(|e| e.to_string())
        };

        Card::new().title("Document").show(ui, |ui| {
            let mut input = forge_egui::widgets::Input::new(&mut self.name)
                .label("Name")
                .placeholder("scratchpad");
            if let Some(err) = name_error {
                input = input.error(err);
            }
            let _ = input.show(ui);

            let mut area = forge_egui::widgets::Textarea::new(&mut self.body)
                .label("JSON body")
                .rows(8);
            if let Some(err) = json_error.as_deref() {
                area = area.error(err);
            }
            let _ = area.show(ui);

            ui.horizontal(|ui| {
                let valid = name_error.is_none() && !self.name.is_empty();
                if Button::new("Save")
                    .variant(Variant::Primary)
                    .disabled(!valid || json_error.is_some() || self.body.trim().is_empty())
                    .show(ui)
                    .clicked()
                {
                    let value: Value = serde_json::from_str(&self.body).unwrap_or(Value::Null);
                    let store = backend.store.clone();
                    let name = self.name.clone();
                    self.put_job = Some(spawn_job(backend, ui.ctx(), async move {
                        store.put(&name, &value).await
                    }));
                }
                if Button::new("Load").disabled(!valid).show(ui).clicked() {
                    let store = backend.store.clone();
                    let name = self.name.clone();
                    self.get_job = Some(spawn_job(backend, ui.ctx(), async move {
                        store.get(&name).await
                    }));
                }
                if Button::new("Delete")
                    .variant(Variant::Danger)
                    .disabled(!valid)
                    .show(ui)
                    .clicked()
                {
                    let store = backend.store.clone();
                    let name = self.name.clone();
                    self.delete_job = Some(spawn_job(backend, ui.ctx(), async move {
                        store.delete(&name).await
                    }));
                }
            });
            if let Some(status) = &self.status {
                ui.label(
                    egui::RichText::new(status)
                        .size(ctx.theme().type_scale.sm)
                        .color(ctx.theme().fg[2]),
                );
            }
        });
        ui.add_space(12.0);

        Card::new().title("Stored documents").show(ui, |ui| {
            ui.horizontal(|ui| {
                if Button::new("Refresh").small(true).show(ui).clicked() {
                    self.refresh(backend, ui.ctx());
                }
            });
            if self.list.is_empty() {
                let _ = Empty::new("No documents")
                    .message("Save one above — it lands in examples/egui-demo/data/")
                    .show(ui);
            }
            let names: Vec<String> = self.list.clone();
            for name in names {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&name)
                            .font(ctx.theme().mono(ctx.theme().type_scale.base))
                            .color(ctx.theme().fg[1]),
                    );
                    if Button::new("Open")
                        .small(true)
                        .variant(Variant::Ghost)
                        .show(ui)
                        .clicked()
                    {
                        self.name = name.clone();
                        let store = backend.store.clone();
                        self.get_job = Some(spawn_job(backend, ui.ctx(), async move {
                            store.get(&name).await
                        }));
                    }
                });
            }
        });
    }

    fn refresh(&mut self, backend: &Backend, egui: &egui::Context) {
        let store = backend.store.clone();
        self.list_job = Some(spawn_job(backend, egui, async move { store.list().await }));
    }

    fn poll(&mut self, ctx: &mut Ctx) {
        if let Some(result) = self.list_job.as_ref().and_then(Job::poll) {
            self.list_job = None;
            match result {
                Ok(metas) => {
                    self.list = metas
                        .iter()
                        .filter_map(|m| m.get("name").and_then(Value::as_str))
                        .map(str::to_owned)
                        .collect();
                }
                Err(e) => ctx.toast().error(format!("list failed: {e}")),
            }
        }
        if let Some(result) = self.get_job.as_ref().and_then(Job::poll) {
            self.get_job = None;
            match result {
                Ok(value) => {
                    self.body = serde_json::to_string_pretty(&value).unwrap_or_default();
                    self.status = Some(format!("loaded '{}'", self.name));
                }
                Err(e) => ctx.toast().error(format!("get failed: {e}")),
            }
        }
        if let Some(result) = self.put_job.as_ref().and_then(Job::poll) {
            self.put_job = None;
            match result {
                Ok(()) => {
                    ctx.toast().success(format!("saved '{}'", self.name));
                    self.listed_once = false; // re-list next frame
                }
                Err(e) => ctx.toast().error(format!("put failed: {e}")),
            }
        }
        if let Some(result) = self.delete_job.as_ref().and_then(Job::poll) {
            self.delete_job = None;
            match result {
                Ok(()) => {
                    ctx.toast().info(format!("deleted '{}'", self.name));
                    self.listed_once = false;
                }
                Err(e) => ctx.toast().error(format!("delete failed: {e}")),
            }
        }
    }
}
