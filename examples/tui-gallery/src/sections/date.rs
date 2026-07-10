use forge_tui::prelude::*;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

const CAL: FocusId = FocusId::new("dt-cal");
const PICKER: FocusId = FocusId::new("dt-picker");

pub struct DateState {
    pub cal: CalendarState,
    pub picker: DatePickerState,
}

impl Default for DateState {
    fn default() -> DateState {
        DateState {
            cal: CalendarState::default(),
            picker: DatePickerState::default(),
        }
    }
}

impl DateState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent, ctx: &mut Ctx) -> Outcome {
        let outcome = match focused {
            Some(id) if id == CAL => self.cal.handle_key(key),
            Some(id) if id == PICKER => self.picker.handle_key(key),
            _ => Outcome::Ignored,
        };
        if outcome == Outcome::Submitted && focused == Some(CAL) {
            ctx.toast().success(format!("Selected {}", self.cal.selected));
            return Outcome::Consumed;
        }
        outcome
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut DateState) {
    let f_cal = ctx.focus.register(CAL);
    let f_picker = ctx.focus.register(PICKER);
    if area.height < 4 {
        return;
    }
    frame.render_widget(
        Eyebrow::new("Calendar — arrows · PgUp/PgDn month · t today").theme(t),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_stateful_widget(
        Calendar::new().focused(f_cal).theme(t),
        Rect::new(area.x, area.y + 1, 22, 9),
        &mut state.cal,
    );

    let px = area.x + 30;
    if px + 16 < area.x + area.width {
        frame.render_widget(
            Eyebrow::new("DatePicker").theme(t),
            Rect::new(px, area.y, area.width - 30, 1),
        );
        frame.render_stateful_widget(
            DatePicker::new().focused(f_picker).theme(t),
            Rect::new(px, area.y + 1, 16, 1),
            &mut state.picker,
        );
    }
}
