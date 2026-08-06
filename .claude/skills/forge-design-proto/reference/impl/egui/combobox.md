# combobox — egui

<!-- RECAST against the #66 template. #64's hand-written "Divergence, unresolved"
     blockquote split cleanly in two: the hint-text trick is Mechanism and always allowed,
     the prefix ranking is a Contract defect. -->

control page: [combobox](../../controls/combobox.md)

## Shape

The field is two different widgets. Closed, it is a click target painted to look like an
input. Open, it is a real `TextEdit::singleline` over `query`. Swapping between them is
the whole reason this page is long.

```rust
pub struct ComboboxState {
    pub open: bool,
    pub query: String,
    pub value: Option<usize>,
}
```

State is plain data the caller owns and passes in. `ComboboxState` has no methods; every
transition happens in `show`.

## What egui gives you

Nothing this control needs. No focus that survives a widget swap, no scroll-into-view, no
key routing you can rely on, and no accessibility. The browser supplies all four to the
SolidJS page, which is why that page is a third of this one.

## Mechanism

**`query` is a `String`, not an optional.** egui has no unset — the field is a text edit
whose buffer *is* `query`, so there is nowhere to put the null. The Contract's
unset-versus-empty distinction is recovered with **hint text**: while `query` is empty,
the selected option's label shows as the hint. The field looks the same and behaves the
same; only the mechanism differs. Do not add an `Option<String>` to get closer to the
SolidJS model — it fights the text edit and buys nothing.

**`value` is an index** into the options slice, not a value. The caller keeps the mapping.

**Focus is recovered by hand.** The widget swap loses focus, because the widget that had
it no longer exists. Use a one-shot flag keyed off the id and request focus on the first
frame the edit exists:

```rust
let focus_id = ui.id().with("combobox-focus");
```

Without this the popup opens and the first keystroke goes nowhere.

## Contract defects

- **Filtering ranks prefix matches first.** The Contract says a plain case-insensitive
  substring match and no ranking. This page is wrong; the ranking has to go, or the
  Contract has to change for all three platforms. ratatui carries the same defect with a
  third answer — see [ratatui](../ratatui/combobox.md).

## Painting the field

Build the frame by hand — `Frame::new()` with `fill` from the raised surface role, a 1px
`Stroke` in the accent role while open, `CornerRadius` from the theme's medium radius, and
a symmetric inner margin of 10.

Set the inner width to the field width minus 20 to account for that margin, and the text
edit's desired width to the available width minus 16 to leave room for the chevron. These
two subtractions are not interchangeable, and getting them wrong clips the caret.

Use `Frame::NONE` on the `TextEdit` itself. Its own frame would draw a second border
inside yours.

## The popup

Use `Popup` with `PopupCloseBehavior::CloseOnClickOutside`, anchored to the field
response. Do not roll a floating area by hand; the popup layer already handles ordering
against modals.

Constrain the popup width to the field width. egui will otherwise size it to its widest
option, and the popup ends up wider than the control.

Scroll is not free. The option list needs an explicit `ScrollArea` once it can exceed the
available height, and keeping the active option visible needs an explicit `scroll_to_rect`
on the frame where `active` changes — not every frame, or the list fights the wheel.

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

Emit `WidgetInfo` with `WidgetType::ComboBox` yourself, including the selected label. Miss
it and the control is invisible to assistive tooling, with no warning at any layer.
