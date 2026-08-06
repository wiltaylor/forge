# combobox — ratatui

<!-- PROTOTYPE (wayfinder #64). -->

status: complete · control page: [combobox](../../controls/combobox.md) ·
index: [ratatui](index.md)

Read the control page first. It owns the state model and the interaction contract.

> **Divergence, unresolved.** The control page makes substring filtering normative. This
> implementation ranks by a fuzzy subsequence score; egui ranks prefix matches first;
> SolidJS is plain substring. Three platforms, three answers, all user-visible.

## Retained state, unlike egui

ratatui redraws every frame but the state struct persists and owns its own behaviour. It
composes the `input` control rather than reimplementing text editing:

```rust
pub struct ComboboxState {
    pub input: InputState,
    pub open: bool,
    highlight: usize,
    filtered: Vec<usize>,
    view_h: usize,
    offset: usize,
    list_area: Rect,
}
```

The last four are private and exist only because a terminal supplies no scrolling. `view_h`
and `offset` are the scroll window, recomputed at draw time from the area given. `list_area`
is stored at draw time so the next mouse event can be hit-tested against it — there is no
DOM to ask.

`filtered` holds indices into the caller's item slice, best match first. Re-ranking runs on
every change, inside the state, so callers never call it themselves.

## Keys

`handle_key` takes the item slice as well as the key, because ranking has to stay current
and the state does not own the items.

Down while closed opens and ranks. Down while open moves `highlight`, stopping at the end.
Up saturates at zero. Enter commits, copies the chosen item into the input, and closes.
Escape closes. Anything else goes to the composed input, and a `Changed` result opens the
popup, re-ranks, and resets `highlight` to zero.

Guard on key **press**. Terminals deliver press and release, and unguarded handlers fire
twice on every key.

Return an `Outcome`: `Ignored`, `Consumed`, `Changed`, or `Submitted`. The value bubbles to
the caller, which is how a screen composes controls without inspecting their internals.

## Mouse

The terminal gives no hit-testing, so this is manual. `handle_mouse` checks the event
against the stored `list_area`, converts the row to an index through `offset`, and treats a
left press as a commit. Wheel deltas move `highlight` rather than scrolling independently —
one cursor, always visible.

## Drawing

Draw the input in place, then the popup. The popup needs `Clear` over its rect first;
without it the content underneath shows through, because a terminal cell has no z-order.

Size the popup to `min(matches, available rows below)`. Do not flip it above the field —
in a terminal the field position is chosen by the layout, and flipping makes the control
jump between frames.

Highlight the active row with a reversed style. Do not use a background colour alone;
256-colour terminals collapse near colours and the row disappears.

Selection is a `>` in the left gutter, not an icon. An icon font is wrong here — this is
the inverse of the graphical platforms.

## What does not exist

No focus ring, no tab order, no ARIA. The caller owns focus and routes keys to whichever
control holds it. This control assumes it has focus whenever `handle_key` is called.
