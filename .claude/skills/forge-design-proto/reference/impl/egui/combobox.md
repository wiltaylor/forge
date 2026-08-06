# combobox — egui

<!-- PROTOTYPE (wayfinder #64). Long on purpose: same control as the SolidJS page, roughly
     three times the length, because egui supplies none of what the browser supplies. -->

status: complete · control page: [combobox](../../controls/combobox.md) ·
index: [egui](index.md)

Read the control page first. It owns the state model and the interaction contract.

> **Divergence, unresolved.** The control page makes substring filtering normative. This
> implementation ranks prefix matches first, and the ratatui one scores a fuzzy
> subsequence. Three platforms, three answers. Filter behaviour is user-visible, so this
> is a real inconsistency, not a rendering detail.

## Immediate mode changes the state model

egui redraws every frame. The control holds no memory of its own, so `ComboboxState` is
plain data the caller owns and passes in:

```rust
pub struct ComboboxState {
    pub open: bool,
    pub query: String,
    pub value: Option<usize>,
}
```

Two consequences the SolidJS page never faces:

`query` is a `String`, not an optional. egui has no "unset" — the field is a text edit
whose buffer *is* `query`, so there is nowhere to put the null. The control page's
unset-versus-empty distinction is recovered with **hint text**: while `query` is empty,
the selected option's label is shown as the hint. The field looks the same; the mechanism
is different. Do not add an `Option<String>` to get closer to the Solid model — it fights
the text edit.

`value` is an index into the options slice, not a value. The caller keeps the mapping.

All state transitions happen in `show`. `ComboboxState` has no methods.

## The field is two different widgets

Closed, it is a click target painted to look like an input. Open, it is a real
`TextEdit::singleline` over `query`. Swapping between them is the whole reason this
control is long.

The swap loses focus, because the widget that had it no longer exists. Recover it with a
one-shot flag keyed off the id, and request focus on the first frame the edit exists:

```rust
let focus_id = ui.id().with("combobox-focus");
```

Without this, the popup opens and the first keystroke goes nowhere.

## Painting the field

Build the frame by hand — `Frame::new()` with `fill` from the raised surface role, a 1px
`Stroke` in the accent role while open, `CornerRadius` from the theme's medium radius, and
a symmetric inner margin of 10.

Set the inner width to the field width minus 20 to account for that margin, and the text
edit's desired width to the available width minus 16 to leave room for the chevron. These
two subtractions are not interchangeable and getting them wrong clips the caret.

Use `Frame::NONE` on the `TextEdit` itself. Its own frame would draw a second border
inside yours.

## The popup

Use `Popup` with `PopupCloseBehavior::CloseOnClickOutside`, anchored to the field
response. Do not roll a floating area by hand; the popup layer already handles ordering
against modals.

Constrain the popup width to the field width. egui will otherwise size it to its widest
option, and the popup ends up wider than the control.

Scroll is not free. The option list needs an explicit `ScrollArea` once it can exceed the
available height, and keeping the active option visible needs an explicit
`scroll_to_rect` on the frame where `active` changes — not every frame, or the list fights
the user's wheel.

## Keys

egui delivers keys to the focused widget, so read them after the text edit and only when
it has focus. Consume Up, Down, Enter and Escape before the text edit sees them, or the
arrows move the caret.

`Key::Escape` also closes the popup layer by default. Handle your own close first and
report the outcome, or one Escape press closes both the popup and whatever is behind it —
the Escape avalanche in [anti-patterns.md](../../anti-patterns.md).

## Outcome

Return `ForgeResponse` carrying an `Outcome`. `Ignored` when nothing happened, `Consumed`
for a state change with no commit, `Submitted` when an option is committed. Callers
compose on this — a screen dispatches on the outcome rather than diffing state, so
returning `Consumed` for a commit silently breaks the caller.

## Accessibility

egui gives you nothing. Emit `WidgetInfo` with `WidgetType::ComboBox` yourself, including
the selected label. Miss it and the control is invisible to assistive tooling, with no
warning at any layer.
