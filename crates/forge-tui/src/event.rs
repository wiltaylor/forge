//! Event plumbing shared by every stateful widget: the [`Outcome`] a state's
//! `handle_key` returns, key-combo matching, and the declarative [`Keymap`]
//! that powers both dispatch and the HelpBar/CommandPalette listings.

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use std::fmt;

/// What a widget state did with a key event. `Ignored` means "not for me" —
/// the caller keeps routing, exactly like DOM event bubbling.
#[must_use = "route the event onward when the widget returns Outcome::Ignored"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Key was not for this widget; keep routing.
    Ignored,
    /// Handled with no observable value change (e.g. cursor move).
    Consumed,
    /// The widget's value or selection changed.
    Changed,
    /// Enter-style commit — read the value from the state.
    Submitted,
    /// Esc-style dismissal.
    Cancelled,
}

impl Outcome {
    pub fn is_handled(self) -> bool {
        self != Outcome::Ignored
    }
}

/// True for the key-event kinds a widget should react to. Windows reports
/// both Press and Release; reacting to Release double-triggers everything.
pub fn is_press(key: &KeyEvent) -> bool {
    matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

/// The pointer position of a mouse event.
pub fn mouse_pos(ev: &MouseEvent) -> Position {
    Position::new(ev.column, ev.row)
}

/// Is the pointer inside `area`?
pub fn in_area(ev: &MouseEvent, area: Rect) -> bool {
    area.contains(mouse_pos(ev))
}

/// Left-button press (the "click" widgets react to).
pub fn left_down(ev: &MouseEvent) -> bool {
    matches!(ev.kind, MouseEventKind::Down(MouseButton::Left))
}

/// Left-button press inside `area` — the standard widget hit test.
pub fn clicked(ev: &MouseEvent, area: Rect) -> bool {
    left_down(ev) && in_area(ev, area)
}

/// Wheel direction: -1 up, +1 down, 0 otherwise.
pub fn scroll_delta(ev: &MouseEvent) -> i32 {
    match ev.kind {
        MouseEventKind::ScrollUp => -1,
        MouseEventKind::ScrollDown => 1,
        _ => 0,
    }
}

/// A key chord: code + modifiers. For character keys SHIFT is ignored during
/// matching because the char itself already carries case/symbol shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

impl KeyCombo {
    pub const fn new(code: KeyCode) -> KeyCombo {
        KeyCombo {
            code,
            mods: KeyModifiers::NONE,
        }
    }

    pub const fn ctrl(code: KeyCode) -> KeyCombo {
        KeyCombo {
            code,
            mods: KeyModifiers::CONTROL,
        }
    }

    pub const fn alt(code: KeyCode) -> KeyCombo {
        KeyCombo {
            code,
            mods: KeyModifiers::ALT,
        }
    }

    pub const fn char(c: char) -> KeyCombo {
        KeyCombo::new(KeyCode::Char(c))
    }

    pub fn matches(&self, key: &KeyEvent) -> bool {
        if !is_press(key) || key.code != self.code {
            return false;
        }
        let strip = |m: KeyModifiers| {
            if matches!(self.code, KeyCode::Char(_)) {
                m.difference(KeyModifiers::SHIFT)
            } else {
                m
            }
        };
        strip(key.modifiers) == strip(self.mods)
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.contains(KeyModifiers::CONTROL) {
            write!(f, "Ctrl+")?;
        }
        if self.mods.contains(KeyModifiers::ALT) {
            write!(f, "Alt+")?;
        }
        if self.mods.contains(KeyModifiers::SHIFT) && !matches!(self.code, KeyCode::Char(_)) {
            write!(f, "Shift+")?;
        }
        match self.code {
            KeyCode::Char(' ') => write!(f, "Space"),
            KeyCode::Char(c) => write!(f, "{}", c.to_uppercase()),
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "Shift+Tab"),
            KeyCode::Backspace => write!(f, "Bksp"),
            KeyCode::Delete => write!(f, "Del"),
            KeyCode::Up => write!(f, "↑"),
            KeyCode::Down => write!(f, "↓"),
            KeyCode::Left => write!(f, "←"),
            KeyCode::Right => write!(f, "→"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "PgUp"),
            KeyCode::PageDown => write!(f, "PgDn"),
            KeyCode::F(n) => write!(f, "F{n}"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// One keymap entry: an action id, the chord that triggers it, and help text.
#[derive(Clone, Debug)]
pub struct Binding {
    pub action: &'static str,
    pub combo: KeyCombo,
    pub help: &'static str,
}

/// Declarative key→action table. Drives dispatch (`action_for`) and the
/// HelpBar / HelpOverlay / CommandPalette listings (`bindings`).
#[derive(Clone, Debug, Default)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

impl Keymap {
    pub fn new() -> Keymap {
        Keymap::default()
    }

    pub fn bind(mut self, action: &'static str, combo: KeyCombo, help: &'static str) -> Keymap {
        self.bindings.push(Binding {
            action,
            combo,
            help,
        });
        self
    }

    pub fn action_for(&self, key: &KeyEvent) -> Option<&'static str> {
        self.bindings
            .iter()
            .find(|b| b.combo.matches(key))
            .map(|b| b.action)
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
}
