# combobox — ratatui

<!-- RECAST against the #66 template. -->

control page: [combobox](../../controls/combobox.md)

## Shape

Retained state that owns its own behaviour, unlike egui. It composes the `input` control
rather than reimplementing text editing:

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

The last four are private and exist only because a terminal supplies no scrolling and no
hit-testing. `view_h` and `offset` are the scroll window, recomputed at draw time from the
area given. `list_area` is stored at draw time so the next mouse event can be tested
against it — there is no DOM to ask.

## What ratatui gives you

A cell grid and a key stream. No focus ring, no tab order, no scrolling, no hit-testing,
no ARIA. The caller owns focus and routes keys to whichever control holds it; this control
assumes it has focus whenever `handle_key` is called.

## Mechanism

**Enter copies the chosen item into the composed input.** The Contract says Enter commits,
closes and clears `query` so the field shows the selected label. There is no separate
label slot in a terminal field, so the same visible result is reached by writing the label
into the input buffer. Same behaviour, different route.

**Selection is a `>` in the left gutter, not a check glyph.** An icon font is wrong here —
this is the inverse of the graphical platforms, where a glyph standing in for an icon is
the anti-pattern.

**The active row is a reversed style, never a background colour alone.** 256-colour
terminals collapse near colours and the row disappears.

## Contract defects

- **Filtering ranks by a fuzzy subsequence score.** The Contract says a plain
  case-insensitive substring match and no ranking. This page is wrong; egui carries the
  same defect with a different answer — see [egui](../egui/combobox.md).

## Keys

`handle_key` takes the item slice as well as the key, because ranking has to stay current
and the state does not own the items.

Down while closed opens and re-filters. Down while open moves `highlight`, stopping at the
end. Up saturates at zero. Enter commits and closes. Escape closes. Anything else goes to
the composed input, and a `Changed` result opens the popup, re-filters, and resets
`highlight` to zero.

Guard on key **press**. Terminals deliver press and release, and unguarded handlers fire
twice on every key.

Return an `Outcome`: `Ignored`, `Consumed`, `Changed`, or `Submitted`. The value bubbles to
the caller, which is how a screen composes controls without inspecting their internals.

## Mouse

Manual. `handle_mouse` checks the event against the stored `list_area`, converts the row
to an index through `offset`, and treats a left press as a commit. Wheel deltas move
`highlight` rather than scrolling independently — one cursor, always visible.

## Drawing

Draw the input in place, then the popup. The popup needs `Clear` over its rect first;
without it the content underneath shows through, because a terminal cell has no z-order.

Size the popup to `min(matches, available rows below)`. Do not flip it above the field. In
a terminal the field position is chosen by the layout, and flipping makes the control jump
between frames — the control page allows the flip, and this platform declines it.
