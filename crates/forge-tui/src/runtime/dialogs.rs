//! Ready-made [`Overlay`] implementations for the runtime stack: confirm
//! dialog, help sheet, floating menu, and command palette. Each hands its
//! outcome back through a shared result cell the caller polls after opening:
//!
//! ```ignore
//! let (dialog, result) = ConfirmDialog::new("Delete node?", "This cannot be undone.");
//! ctx.open(Box::new(dialog));
//! // later (tick/draw): if let Some(true) = result.take() { delete(); }
//! ```

use crate::event::{is_press, Keymap, Outcome};
use crate::runtime::{Overlay, OverlayOutcome};
use crate::text;
use crate::theme::Theme;
use crate::widgets::overlays::{Command, DropdownMenu, MenuEntry, MenuState, Modal, Palette, PaletteState};
use crate::widgets::primitives::{Button, Variant};
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Frame;
use std::cell::Cell;
use std::rc::Rc;

/// Shared single-value result slot for overlay dialogs.
pub struct DialogResult<T>(Rc<Cell<Option<T>>>);

impl<T> Clone for DialogResult<T> {
    fn clone(&self) -> Self {
        DialogResult(self.0.clone())
    }
}

impl<T> DialogResult<T> {
    fn new() -> (DialogResult<T>, DialogResult<T>) {
        let inner = Rc::new(Cell::new(None));
        (DialogResult(inner.clone()), DialogResult(inner))
    }

    fn set(&self, value: T) {
        self.0.set(Some(value));
    }

    /// Take the result if the dialog has resolved.
    pub fn take(&self) -> Option<T> {
        self.0.take()
    }
}

/// Prebuilt confirm modal: message + Cancel/Confirm buttons, `y`/`n`
/// shortcuts, ←/→/Tab to switch, Enter to commit, Esc cancels.
pub struct ConfirmDialog {
    title: String,
    message: String,
    confirm_label: String,
    danger: bool,
    focus_confirm: bool,
    result: DialogResult<bool>,
}

impl ConfirmDialog {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> (ConfirmDialog, DialogResult<bool>) {
        let (a, b) = DialogResult::new();
        (
            ConfirmDialog {
                title: title.into(),
                message: message.into(),
                confirm_label: "Confirm".into(),
                danger: false,
                focus_confirm: false,
                result: a,
            },
            b,
        )
    }

    pub fn confirm_label(mut self, label: impl Into<String>) -> ConfirmDialog {
        self.confirm_label = label.into();
        self
    }

    /// Style the confirm button as destructive.
    pub fn danger(mut self) -> ConfirmDialog {
        self.danger = true;
        self
    }
}

impl Overlay for ConfirmDialog {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let body = text::wrap(&self.message, 46);
        let h = (body.len() as u16 + 5).max(7);
        let modal = Modal::new()
            .title(&self.title)
            .footer(" Esc cancel · Enter confirm ")
            .size(52, h)
            .theme(theme);
        let inner = modal.inner(area);
        frame.render_widget(modal, area);
        let buf = frame.buffer_mut();
        for (i, line) in body.iter().enumerate() {
            let y = inner.y + i as u16;
            if y + 2 >= inner.y + inner.height {
                break;
            }
            buf.set_string(inner.x, y, line, Style::new().fg(theme.fg[1]).bg(theme.bg[4]));
        }
        let by = inner.y + inner.height.saturating_sub(1);
        let cancel = Button::new("Cancel").focused(!self.focus_confirm).theme(theme);
        let confirm = Button::new(&self.confirm_label)
            .variant(if self.danger { Variant::Danger } else { Variant::Primary })
            .focused(self.focus_confirm)
            .theme(theme);
        let cw = confirm.width();
        let caw = cancel.width();
        let total = cw + caw + 2;
        let bx = inner.x + inner.width.saturating_sub(total);
        frame.render_widget(cancel, Rect::new(bx, by, caw, 1));
        frame.render_widget(confirm, Rect::new(bx + caw + 2, by, cw, 1));
    }

    fn handle(&mut self, event: &Event) -> OverlayOutcome {
        let Event::Key(key) = event else {
            return OverlayOutcome::Consumed;
        };
        if !is_press(key) {
            return OverlayOutcome::Consumed;
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.result.set(true);
                OverlayOutcome::Close
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.result.set(false);
                OverlayOutcome::Close
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                self.focus_confirm = !self.focus_confirm;
                OverlayOutcome::Consumed
            }
            KeyCode::Enter => {
                self.result.set(self.focus_confirm);
                OverlayOutcome::Close
            }
            KeyCode::Esc => {
                self.result.set(false);
                OverlayOutcome::Close
            }
            _ => OverlayOutcome::Consumed,
        }
    }
}

/// Keybinding cheatsheet modal built from a [`Keymap`] (bind it to `?`).
pub struct HelpOverlay {
    title: String,
    rows: Vec<(String, String)>,
}

impl HelpOverlay {
    pub fn new(keymap: &Keymap) -> HelpOverlay {
        HelpOverlay {
            title: "Keyboard shortcuts".into(),
            rows: keymap
                .bindings()
                .iter()
                .map(|b| (b.combo.to_string(), b.help.to_string()))
                .collect(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> HelpOverlay {
        self.title = title.into();
        self
    }
}

impl Overlay for HelpOverlay {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let h = (self.rows.len() as u16 + 4).min(area.height);
        let modal = Modal::new()
            .title(&self.title)
            .footer(" Esc close ")
            .size(44, h)
            .theme(theme);
        let inner = modal.inner(area);
        frame.render_widget(modal, area);
        let buf = frame.buffer_mut();
        let kbd_w = self
            .rows
            .iter()
            .map(|(k, _)| text::width(k))
            .max()
            .unwrap_or(0) as u16;
        for (i, (kbd, help)) in self.rows.iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            buf.set_string(inner.x, y, kbd, Style::new().fg(theme.fg[0]).bg(theme.bg[3]));
            buf.set_string(
                inner.x + kbd_w + 2,
                y,
                text::truncate(help, inner.width.saturating_sub(kbd_w + 2) as usize),
                Style::new().fg(theme.fg[1]).bg(theme.bg[4]),
            );
        }
    }

    fn handle(&mut self, event: &Event) -> OverlayOutcome {
        if let Event::Key(key) = event {
            if is_press(key) && matches!(key.code, KeyCode::Char('?') | KeyCode::Char('q')) {
                return OverlayOutcome::Close;
            }
        }
        OverlayOutcome::Ignored // Esc-close via stack default
    }
}

/// Owned menu entry for [`MenuOverlay`].
#[derive(Clone, Debug)]
pub enum OwnedMenuEntry {
    Item {
        label: String,
        kbd: Option<String>,
        danger: bool,
        disabled: bool,
    },
    Section(String),
    Separator,
}

impl OwnedMenuEntry {
    pub fn item(label: impl Into<String>) -> OwnedMenuEntry {
        OwnedMenuEntry::Item { label: label.into(), kbd: None, danger: false, disabled: false }
    }

    pub fn item_kbd(label: impl Into<String>, kbd: impl Into<String>) -> OwnedMenuEntry {
        OwnedMenuEntry::Item {
            label: label.into(),
            kbd: Some(kbd.into()),
            danger: false,
            disabled: false,
        }
    }

    pub fn danger(label: impl Into<String>) -> OwnedMenuEntry {
        OwnedMenuEntry::Item { label: label.into(), kbd: None, danger: true, disabled: false }
    }

    pub fn section(title: impl Into<String>) -> OwnedMenuEntry {
        OwnedMenuEntry::Section(title.into())
    }

    pub fn separator() -> OwnedMenuEntry {
        OwnedMenuEntry::Separator
    }
}

/// Floating dropdown/context menu as a stack overlay. Resolves to the
/// selected *selectable-item* index (`None` on dismiss).
pub struct MenuOverlay {
    entries: Vec<OwnedMenuEntry>,
    anchor: Rect,
    state: MenuState,
    result: DialogResult<Option<usize>>,
}

impl MenuOverlay {
    pub fn new(entries: Vec<OwnedMenuEntry>, anchor: Rect) -> (MenuOverlay, DialogResult<Option<usize>>) {
        let (a, b) = DialogResult::new();
        (
            MenuOverlay { entries, anchor, state: MenuState::new(), result: a },
            b,
        )
    }

}

fn borrow_entries(entries: &[OwnedMenuEntry]) -> Vec<MenuEntry<'_>> {
    entries
        .iter()
        .map(|e| match e {
            OwnedMenuEntry::Item { label, kbd, danger, disabled } => MenuEntry::Item {
                label,
                kbd: kbd.as_deref(),
                danger: *danger,
                disabled: *disabled,
            },
            OwnedMenuEntry::Section(s) => MenuEntry::Section(s),
            OwnedMenuEntry::Separator => MenuEntry::Separator,
        })
        .collect()
}

impl Overlay for MenuOverlay {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let entries = borrow_entries(&self.entries);
        let menu = DropdownMenu::new(&entries, self.anchor).theme(theme);
        frame.render_stateful_widget(menu, area, &mut self.state);
    }

    fn handle(&mut self, event: &Event) -> OverlayOutcome {
        let Event::Key(key) = event else {
            return OverlayOutcome::Consumed;
        };
        let outcome = match key.code {
            KeyCode::Char(c) if is_press(key) => {
                let entries = borrow_entries(&self.entries);
                self.state.mnemonic(&entries, c)
            }
            _ => self.state.handle_key(*key),
        };
        match outcome {
            Outcome::Submitted => {
                self.result.set(Some(self.state.highlight));
                OverlayOutcome::Close
            }
            Outcome::Cancelled => {
                self.result.set(None);
                OverlayOutcome::Close
            }
            _ => OverlayOutcome::Consumed,
        }
    }

    fn dim_below(&self) -> bool {
        false // menus float without a scrim, like the web dropdowns
    }
}

/// Owned palette command for [`PaletteOverlay`].
#[derive(Clone, Debug)]
pub struct OwnedCommand {
    pub id: String,
    pub label: String,
    pub kbd: Option<String>,
}

impl OwnedCommand {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> OwnedCommand {
        OwnedCommand { id: id.into(), label: label.into(), kbd: None }
    }

    pub fn kbd(mut self, kbd: impl Into<String>) -> OwnedCommand {
        self.kbd = Some(kbd.into());
        self
    }
}

/// Ctrl+K command palette as a stack overlay; resolves to the chosen command
/// id (`None` on dismiss).
pub struct PaletteOverlay {
    commands: Vec<OwnedCommand>,
    state: PaletteState,
    result: DialogResult<Option<String>>,
}

impl PaletteOverlay {
    pub fn new(commands: Vec<OwnedCommand>) -> (PaletteOverlay, DialogResult<Option<String>>) {
        let (a, b) = DialogResult::new();
        (
            PaletteOverlay { commands, state: PaletteState::new(), result: a },
            b,
        )
    }

}

fn borrow_commands(commands: &[OwnedCommand]) -> Vec<Command<'_>> {
    commands
        .iter()
        .map(|c| Command { id: &c.id, label: &c.label, kbd: c.kbd.as_deref() })
        .collect()
}

impl Overlay for PaletteOverlay {
    fn draw(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let commands = borrow_commands(&self.commands);
        let palette = Palette::new(&commands).theme(theme);
        frame.render_stateful_widget(palette, area, &mut self.state);
    }

    fn handle(&mut self, event: &Event) -> OverlayOutcome {
        match event {
            Event::Key(key) => {
                let commands = borrow_commands(&self.commands);
                match self.state.handle_key(*key, &commands) {
                    Outcome::Submitted => {
                        let id = self
                            .state
                            .highlighted()
                            .map(|i| self.commands[i].id.clone());
                        self.result.set(id);
                        OverlayOutcome::Close
                    }
                    Outcome::Cancelled => {
                        self.result.set(None);
                        OverlayOutcome::Close
                    }
                    _ => OverlayOutcome::Consumed,
                }
            }
            Event::Paste(s) => {
                self.state.input.insert_str(s);
                let commands = borrow_commands(&self.commands);
                self.state.filter(&commands);
                OverlayOutcome::Consumed
            }
            _ => OverlayOutcome::Consumed,
        }
    }
}
