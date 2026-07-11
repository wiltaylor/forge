//! File picker: filesystem browsing with breadcrumbs and a hidden-files
//! toggle. Starts at the repo root.

use forge_egui::prelude::*;
use forge_egui::widgets::{FilePicker, FilePickerState};
use std::path::Path;

pub struct FilesState {
    pub picker: FilePickerState,
}

impl Default for FilesState {
    fn default() -> Self {
        // examples/egui-gallery → repo root, two levels up.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap_or_else(|| Path::new("."));
        FilesState {
            picker: FilePickerState::new(root),
        }
    }
}

pub fn draw(ui: &mut egui::Ui, state: &mut FilesState) {
    let t = Theme::of(ui.ctx());

    Card::new().title("File picker").show(ui, |ui| {
        let r = FilePicker::new(&mut state.picker).height(320.0).show(ui);
        if r.submitted() {
            // The chosen path is in `state.picker.selected`.
        }
        ui.add_space(8.0);
        let readout = match &state.picker.selected {
            Some(path) => format!("selected: {}", path.display()),
            None => "selected: — (double-click or Enter on a file)".to_owned(),
        };
        ui.label(
            egui::RichText::new(readout)
                .font(t.mono(t.type_scale.sm))
                .color(t.fg[2]),
        );
    });
}
