# table — egui

<!-- RECAST against the #66 template. -->

control page: [table](../../controls/table.md)

## Shape

Do not use `egui::Grid`. It has no sticky header, no row hit area, and it sizes columns
from content on every frame, so the table jitters as data changes.

Paint it: allocate the header strip, then a `ScrollArea` for the body, then rows.

## What egui gives you

A scroll area and input sensing. No sticky positioning, no per-cell clipping, no
truncation, and no accessibility.

## Mechanism

**The header sits outside the `ScrollArea`.** There is no sticky positioning; keeping the
header inside and trying to pin it does not work. This is how the Contract's "header stays
visible while the body scrolls" is met here.

**Resolve column widths once per frame** from the caller's constraints against the
available width, and store the resolved widths in the state so the header and body agree.
Resolving them twice — once for the header, once for the body — is the standard way to get
a header that drifts out of alignment as the scrollbar appears.

**Reserve the scrollbar width in the header** even when no scrollbar is showing, or the
header shifts by that width the moment the body overflows.

**Truncate by measuring the galley** against the column width and re-laying it out with an
ellipsis. `Label::truncate` is close but does not honour the Forge ellipsis position on
every font.

## Rows

Allocated at the control height and sensed for `click`. Paint the row background first
(active takes the raised surface, selected takes the accent at low alpha), then clip each
cell to its column rect before painting the galley. Without the per-cell clip, long text
paints over the next column — egui does not clip for you.

## Keyboard

The table takes one focus id. Read Up, Down, PageUp, PageDown, Home and End on the frame
it has focus. Keep the active row visible with `scroll_to_rect` on the frame the active
row changes — not every frame, or the wheel stops working.

## Accessibility

Emit `WidgetInfo` for the table and for the active row. Nothing is emitted for you.

## Outcome

Return `ForgeResponse`: `Changed` when the active row or the sort moves, `Submitted` on
Enter, `Ignored` otherwise.
