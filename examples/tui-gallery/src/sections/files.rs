use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const PICKER: FocusId = FocusId::new("fp-picker");

pub struct FilesState {
    pub picker: FilePickerState,
}

impl Default for FilesState {
    fn default() -> FilesState {
        FilesState {
            picker: FilePickerState::new(std::env::current_dir().unwrap_or_else(|_| "/".into())),
        }
    }
}

impl FilesState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        if focused != Some(PICKER) {
            return Outcome::Ignored;
        }
        let outcome = self.picker.handle_key(key);
        if outcome == Outcome::Submitted {
            if let Some(path) = self.picker.take_selected() {
                ctx.toast().success(format!("Picked {}", path.display()));
            }
            return Outcome::Consumed;
        }
        outcome
    }
}

impl FilesState {
    pub fn handle_mouse(&mut self, ev: &MouseEvent, ctx: &mut Ctx) -> Outcome {
        let out = self.picker.handle_mouse(ev);
        if out.is_handled() {
            ctx.focus.focus(PICKER);
            if out == Outcome::Submitted {
                if let Some(path) = self.picker.take_selected() {
                    ctx.toast().success(format!("Picked {}", path.display()));
                }
                return Outcome::Consumed;
            }
        }
        out
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut FilesState) {
    let focused = ctx.focus.register(PICKER);
    if area.height < 3 {
        return;
    }
    frame.render_widget(
        Eyebrow::new("FilePicker — Enter descend/pick · Bksp up · . hidden").theme(t),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_stateful_widget(
        FilePicker::new().focused(focused).theme(t),
        Rect::new(area.x, area.y + 1, area.width.min(60), area.height - 1),
        &mut state.picker,
    );
}
