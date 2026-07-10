use forge_tui::event::Outcome;
use forge_tui::runtime::{FocusId, FocusRing, Overlay, OverlayOutcome, OverlayStack};
use forge_tui::theme::Theme;
use forge_tui::widgets::{
    CheckboxState, InputState, RadioGroup, RadioState, ToggleState,
};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn type_str(state: &mut InputState, s: &str) {
    for c in s.chars() {
        assert_eq!(state.handle_key(key(KeyCode::Char(c))), Outcome::Changed);
    }
}

#[test]
fn input_typing_and_editing() {
    let mut s = InputState::new();
    type_str(&mut s, "hello world");
    assert_eq!(s.value(), "hello world");

    assert_eq!(s.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert_eq!(s.value(), "hello worl");

    // Ctrl+W deletes the previous word.
    assert_eq!(s.handle_key(ctrl('w')), Outcome::Changed);
    assert_eq!(s.value(), "hello ");

    // Ctrl+U kills to start.
    assert_eq!(s.handle_key(ctrl('u')), Outcome::Changed);
    assert_eq!(s.value(), "");

    // Backspace at start is consumed but changes nothing.
    assert_eq!(s.handle_key(key(KeyCode::Backspace)), Outcome::Consumed);
}

#[test]
fn input_cursor_and_word_movement() {
    let mut s = InputState::with_value("alpha beta");
    assert_eq!(s.cursor(), 10);
    s.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL))
        .is_handled()
        .then_some(())
        .unwrap();
    assert_eq!(s.cursor(), 6); // start of "beta"
    s.handle_key(key(KeyCode::Home)).is_handled().then_some(()).unwrap();
    assert_eq!(s.cursor(), 0);
    s.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL))
        .is_handled()
        .then_some(())
        .unwrap();
    assert_eq!(s.cursor(), 5); // end of "alpha"

    // Ctrl+K kills to end from here.
    assert_eq!(s.handle_key(ctrl('k')), Outcome::Changed);
    assert_eq!(s.value(), "alpha");
}

#[test]
fn input_selection_replaces_on_type() {
    let mut s = InputState::with_value("abc");
    let shift_left = KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT);
    s.handle_key(shift_left).is_handled().then_some(()).unwrap();
    s.handle_key(shift_left).is_handled().then_some(()).unwrap();
    assert_eq!(s.selection(), Some((1, 3)));
    assert_eq!(s.handle_key(key(KeyCode::Char('X'))), Outcome::Changed);
    assert_eq!(s.value(), "aX");
    assert_eq!(s.selection(), None);
}

#[test]
fn input_handles_multibyte_graphemes() {
    let mut s = InputState::new();
    type_str(&mut s, "aé日");
    assert_eq!(s.value(), "aé日");
    assert_eq!(s.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert_eq!(s.value(), "aé");
    assert_eq!(s.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert_eq!(s.value(), "a");
}

#[test]
fn input_submit_and_cancel() {
    let mut s = InputState::with_value("x");
    assert_eq!(s.handle_key(key(KeyCode::Enter)), Outcome::Submitted);
    assert_eq!(s.handle_key(key(KeyCode::Esc)), Outcome::Cancelled);
    // Tab is not the input's business.
    assert_eq!(s.handle_key(key(KeyCode::Tab)), Outcome::Ignored);
}

#[test]
fn checkbox_and_toggle_toggle_on_space() {
    let mut c = CheckboxState::new(false);
    assert_eq!(c.handle_key(key(KeyCode::Char(' '))), Outcome::Changed);
    assert!(c.checked);
    let mut t = ToggleState::new(true);
    assert_eq!(t.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    assert!(!t.on);
    assert_eq!(t.handle_key(key(KeyCode::Char('x'))), Outcome::Ignored);
}

#[test]
fn radio_navigation_clamps_to_rendered_len() {
    let mut state = RadioState::new(0);
    // Render once so the state learns the item count.
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 3));
    RadioGroup::new(&["a", "b", "c"]).render(Rect::new(0, 0, 20, 3), &mut buf, &mut state);

    assert_eq!(state.handle_key(key(KeyCode::Down)), Outcome::Changed);
    assert_eq!(state.selected, 1);
    assert_eq!(state.handle_key(key(KeyCode::Down)), Outcome::Changed);
    assert_eq!(state.handle_key(key(KeyCode::Down)), Outcome::Consumed); // clamped
    assert_eq!(state.selected, 2);
    assert_eq!(state.handle_key(key(KeyCode::Up)), Outcome::Changed);
    assert_eq!(state.selected, 1);
}

#[test]
fn focus_ring_traverses_in_render_order_and_wraps() {
    let a = FocusId::new("a");
    let b = FocusId::new("b");
    let c = FocusId::indexed("row", 3);
    let mut ring = FocusRing::new();

    // Frame 1: register a, b, c. First registered wins initial focus.
    ring.begin_frame();
    assert!(ring.register(a));
    assert!(!ring.register(b));
    assert!(!ring.register(c));

    ring.begin_frame();
    ring.register(a);
    ring.register(b);
    ring.register(c);

    ring.next();
    assert!(ring.is(b));
    ring.next();
    assert!(ring.is(c));
    ring.next();
    assert!(ring.is(a)); // wrapped
    ring.prev();
    assert!(ring.is(c)); // wrapped backwards
}

#[test]
fn focus_ring_recovers_when_focused_widget_disappears() {
    let a = FocusId::new("a");
    let b = FocusId::new("b");
    let mut ring = FocusRing::new();
    ring.begin_frame();
    ring.register(a);
    ring.register(b);
    ring.focus(b);
    // New frame no longer renders b.
    ring.begin_frame();
    ring.register(a);
    ring.begin_frame();
    ring.register(a);
    ring.next();
    assert!(ring.is(a));
}

struct DummyOverlay {
    outcome: OverlayOutcome,
}

impl Overlay for DummyOverlay {
    fn draw(&mut self, _f: &mut ratatui::Frame, _a: Rect, _t: &Theme) {}
    fn handle(&mut self, _e: &Event) -> OverlayOutcome {
        self.outcome
    }
}

#[test]
fn overlay_stack_esc_closes_by_default_and_swallows_events() {
    let mut stack = OverlayStack::new();
    assert!(!stack.handle(&Event::Key(key(KeyCode::Esc)))); // empty: not swallowed

    stack.push(Box::new(DummyOverlay { outcome: OverlayOutcome::Ignored }));
    stack.push(Box::new(DummyOverlay { outcome: OverlayOutcome::Consumed }));
    assert_eq!(stack.len(), 2);

    // Top consumes: stays open, event swallowed.
    assert!(stack.handle(&Event::Key(key(KeyCode::Esc))));
    assert_eq!(stack.len(), 2);

    stack.pop();
    // Now the top ignores: default Esc-close pops it.
    assert!(stack.handle(&Event::Key(key(KeyCode::Esc))));
    assert_eq!(stack.len(), 0);
}
