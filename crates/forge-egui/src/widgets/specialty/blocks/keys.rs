//! The block editor's key adapter: egui input in, shared [`Op`] out.
//!
//! The editing policy is not here. [`forge_blocks::resolve_key`] owns it —
//! what a keypress means, given the document, the address and what the
//! editor is doing with the block. This module is the half only a kit can
//! write: turning egui's input into the normalised [`Key`] shape, and
//! turning the [`Op`] that comes back into this editor's own bookkeeping.
//!
//! **Where the character comes from.** egui reports a keypress twice: an
//! [`Event::Key`] naming the key and carrying the modifiers, and — when the
//! key produced a character — an [`Event::Text`] carrying that character and
//! nothing else. So this adapter reads `code` and the modifiers off the key
//! event and the produced character off the text event, which is exactly the
//! split [`Key`] describes. egui suppresses the text event for a Ctrl or Cmd
//! chord, so the resolver's rules that need a character *and* a modifier
//! (Ctrl+T, Alt+=, Alt+-) do not reach this kit — the same gap
//! `contract/blocks/corpus.json` already records for the egui table ops,
//! which are toolbar buttons under the grid.
//!
//! **What "consumed" means.** A keypress is taken out of egui's queue only
//! when this editor acts on it. Everything else is left for the focused
//! widget, because the buffer and its caret are the kit's half of the deal:
//! [`Op::Insert`] is performed by letting the `TextEdit` type the character
//! it was already going to type, and caret motion never reaches the resolver
//! at all.

use egui::{Context, Event, Id, Ui};
use forge_blocks::{resolve_key, Address, Document, Key, Mode, Op};

use super::{Action, BlockEditorState, CaretHint, Ecx, SlashState};

/// What the keyboard is pointed at: the block, what the editor is doing with
/// it, and the buffer holding it open.
pub(super) struct Focused {
    /// The block the keys go to.
    pub addr: Address,
    /// What the editor is doing with it — the resolver's four modes.
    pub mode: Mode,
    /// The egui id of the buffer that owns the keyboard, when one does.
    /// Leaving a buffer means giving its focus back, which only the widget
    /// that made it can name.
    pub buffer: Option<Id>,
    /// Whether that buffer holds a selection. Backspace and Delete are then
    /// the buffer's — deleting the selection — whatever the resolver says,
    /// because a selection makes the caret a range the resolver cannot read.
    pub selection: bool,
}

/// Read the next keypress this editor acts on, resolve it against the shared
/// policy, and perform what comes back. Says whether a key resolved to
/// something the editor acted on — which is not the same as consuming it.
///
/// Scanning stops at the first key the editor takes; keys before it belong to
/// the focused buffer (a typed character, a caret move) and are left in the
/// queue for it.
pub(super) fn handle(
    ui: &Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &Document,
    focused: Focused,
) -> bool {
    // Pick inside one input borrow: an egui context is a single lock, so
    // nothing in here may reach back into it.
    let picked = ui.ctx().input(|i| {
        for event in &i.events {
            let Some(key) = normalize(event) else {
                continue;
            };
            if focused.selection && matches!(key.code.as_str(), "Backspace" | "Delete") {
                continue;
            }
            match resolve_key(doc, focused.addr, focused.mode, &key) {
                // The character the resolver would type is the character the
                // focused `TextEdit` is about to type from this very event.
                // Walking past it is how this kit performs the op.
                Some(Op::Insert(_)) => continue,
                Some(op) => return Some((event.clone(), Some(op))),
                // Tab is never markdown. The resolver leaves it unbound off a
                // list item, and the multiline `TextEdit` holding the source
                // would answer by typing a tab character into it. A code or
                // JSON body is a different buffer with a different answer —
                // there, a tab is exactly what Tab means.
                None if key.code == "Tab" && matches!(focused.mode, Mode::Text { .. }) => {
                    return Some((event.clone(), None))
                }
                None => continue,
            }
        }
        None
    });

    let Some((event, op)) = picked else {
        return false;
    };
    let spent = match op {
        Some(op) => perform(ui.ctx(), ecx, st, &focused, op),
        None => true,
    };
    if spent {
        drop_event(ui.ctx(), &event);
    }
    true
}

/// The same, for a buffer of the kit's own holding a block's content — a
/// code body, a data block's JSON source. The block-level keys still apply;
/// every other key is the buffer's.
pub(super) fn buffer(
    ui: &Ui,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    doc: &Document,
    addr: Address,
    body: Id,
) {
    handle(
        ui,
        ecx,
        st,
        doc,
        Focused {
            addr,
            mode: Mode::Buffer,
            buffer: Some(body),
            selection: false,
        },
    );
}

/// `consume_key` for a key held with no modifier at all.
///
/// egui's own ignores *extra* Shift and Alt, so a popup asking for `↑` would
/// eat the Alt+↑ that moves the block. Every key this kit takes before the
/// resolver is asked for exactly, so the resolver still sees the chords.
pub(super) fn consume_plain(ui: &Ui, want: egui::Key) -> bool {
    ui.ctx().input_mut(|i| {
        let mut hit = false;
        i.events.retain(|event| {
            let is_match = matches!(
                event,
                Event::Key { key, pressed: true, modifiers, .. }
                    if *key == want && modifiers.is_none()
            );
            hit |= is_match;
            !is_match
        });
        hit
    })
}

/// One egui event in the shared key shape, or `None` when it is not a
/// keypress this vocabulary can name (a released key, a paste, a pointer
/// move, an IME string, a key with no `KeyboardEvent.code`).
fn normalize(event: &Event) -> Option<Key> {
    match event {
        Event::Text(text) => {
            let mut chars = text.chars();
            let c = chars.next()?;
            // One character is a keypress; more is an IME commit or a
            // paste-alike, which belongs to the buffer.
            chars.next().is_none().then(|| Key::typed(c))
        }
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let mut named = Key::new(crate::keys::code_str(*key)?);
            named.shift = modifiers.shift;
            named.ctrl = modifiers.ctrl || modifiers.command;
            named.alt = modifiers.alt;
            Some(named)
        }
        _ => None,
    }
}

/// Take a keypress out of egui's queue. Matching on the event rather than an
/// index keeps this honest if anything else has touched the queue since.
fn drop_event(ctx: &Context, event: &Event) {
    ctx.input_mut(|i| {
        if let Some(at) = i.events.iter().position(|pending| pending == event) {
            i.events.remove(at);
        }
    });
}

/// Perform one resolved op, and say whether it spends the keypress.
///
/// Structural edits are deferred as [`Action`]s — block indices have to stay
/// valid while the walk is rendering. Everything the same frame still needs
/// (the table's pending cell, the palette) is set here and now.
fn perform(
    ctx: &Context,
    ecx: &mut Ecx,
    st: &mut BlockEditorState,
    focused: &Focused,
    op: Op,
) -> bool {
    let addr = focused.addr;
    let mut act = |action| {
        ecx.actions.push(action);
        true
    };
    match op {
        // Bound, with nothing to do: Delete at the end of the last block, a
        // selection step off the end of the document.
        Op::Nothing => true,
        // Handled by the scan — kept exhaustive so a new op cannot slip past.
        Op::Insert(_) => false,
        Op::Split { caret } => act(Action::Split { addr, caret }),
        Op::Demote { addr } => act(Action::Demote(addr)),
        Op::Merge { addr } => act(Action::Merge(addr)),
        Op::Convert { kind, caret } => act(Action::Shortcut { addr, kind, caret }),
        Op::Indent { delta } => act(Action::Indent { addr, delta }),
        Op::MoveBlock { dir } => act(Action::MoveBlock { addr, dir }),
        Op::Remove => act(Action::Remove(addr)),
        Op::CycleTone => act(Action::CycleTone(addr)),
        Op::WrapColumns { n } => act(Action::WrapColumns { addr, n }),
        Op::Select { addr } => {
            surrender(ctx, focused.buffer);
            act(Action::Select(addr))
        }
        Op::Blur => {
            surrender(ctx, focused.buffer);
            st.focus = None;
            true
        }
        Op::Enter => act(Action::Focus(addr, CaretHint::End)),
        // The palette is a popup inside the focused text block's `TextEdit`,
        // filtered by everything after the leading `/` in its draft — so the
        // `/` goes on into that draft rather than being spent here.
        //
        // A gap, not a policy: with a block merely selected there is no draft
        // and no `TextEdit` to anchor to, so `/` opens nothing. The ratatui
        // kit binds it in both modes. Closing it means making the palette a
        // popup of the editor rather than of one text block, which is a
        // change to this kit's UI, not to its key handling.
        Op::OpenPalette => {
            if matches!(focused.mode, Mode::Text { .. }) {
                st.slash = Some(SlashState { addr, hl: 0 });
            }
            false
        }
        Op::FocusCell { row, col } => {
            st.pending_cell = Some((row, col));
            true
        }
        Op::InsertRow {
            at,
            focus: Some(focus),
        } => act(Action::InsertTableRow { addr, at, focus }),
        // The table ops that are not "Enter off the last row" have no
        // keyboard home in this kit: `+ Row` / `− Row` / `+ Col` / `− Col`
        // are toolbar buttons under the grid, and Ctrl+Enter is bound to
        // nothing at all. `contract/blocks/corpus.json` records exactly that,
        // in the four table cases it marks inapplicable to `rust-egui`.
        // Binding them here would make those notes false while leaving the
        // cases unrun, so they stop at the adapter.
        Op::InsertRow { .. } | Op::InsertCol { .. } | Op::RemoveCol { .. } => false,
    }
}

/// Give the keyboard back, so the block that just stopped being edited does
/// not leave a vanished `TextEdit` holding egui's focus.
fn surrender(ctx: &Context, buffer: Option<Id>) {
    if let Some(id) = buffer {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Modifiers;

    fn text(s: &str) -> Event {
        Event::Text(s.to_owned())
    }

    fn pressed(key: egui::Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn a_named_key_carries_its_code_and_modifiers() {
        let key = normalize(&pressed(egui::Key::ArrowUp, Modifiers::ALT)).unwrap();
        assert_eq!(key.code, "ArrowUp");
        assert!(key.alt && !key.ctrl && !key.shift);
        assert_eq!(key.key, None);
    }

    #[test]
    fn a_typed_character_carries_the_shift_its_layout_implies() {
        let key = normalize(&text("#")).unwrap();
        assert_eq!(key.code, "Digit3");
        assert_eq!(key.key.as_deref(), Some("#"));
        assert!(key.shift);
    }

    /// Everything that is not a keypress this vocabulary names is left for
    /// the focused widget.
    #[test]
    fn non_keypresses_are_left_alone() {
        assert!(normalize(&text("ありがとう")).is_none());
        assert!(normalize(&Event::Paste("x".to_owned())).is_none());
        assert!(normalize(&pressed(egui::Key::F13, Modifiers::NONE)).is_none());
        assert!(normalize(&Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        })
        .is_none());
    }
}
