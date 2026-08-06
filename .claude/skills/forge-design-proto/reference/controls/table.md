# table

<!-- PROTOTYPE (wayfinder #64). Chosen as the third filled control because its platform
     renderings diverge most — the browser supplies scroll and sticky headers, ratatui and
     egui supply neither. -->

Implementations: [solid](../impl/solid/table.md) · [ratatui](../impl/ratatui/table.md) ·
[egui](../impl/egui/table.md)

## What it is for

Rows of records with the same shape, compared across columns. `logs` when the rows are a
stream and the newest matters most. `list-box` when there is one column and the point is
selection.

## Anatomy

A header row, then body rows. Optional leading selection column. Optional trailing actions
column, `ghost` icon-buttons only.

The header is `surface-sunken`, `text-dim`, and stays visible while the body scrolls.

Rows are 32px and do not grow. Content that does not fit truncates with an ellipsis at the
end. A row is never two lines.

## Column alignment

Text left. Numbers right, tabular, with the unit. Status glyphs centred. This is not a
preference — a right-aligned number column is how the eye compares magnitudes.

## State

| State | Meaning |
|---|---|
| `rows` | The data. Caller-owned |
| `sort` | Column and direction, or none |
| `selection` | Selected row keys. Caller-owned |
| `active` | The keyboard row |

Sorting is requested, never performed. The table reports which column and direction the
user asked for; the caller sorts and passes new `rows` back. A table that sorts its own
copy desynchronises from the caller's paging.

## Interaction contract

- Up and Down move `active` by a row, and stop at the ends without wrapping.
- Page Up and Page Down move by the visible row count.
- Home and End move to the first and last row.
- Enter activates the `active` row, if the caller supplied an activation handler.
- Space toggles selection on the `active` row, when selection is on.
- Activating a header cell cycles its sort: ascending, descending, none.
- Scrolling follows `active` — the active row is always visible.

## Empty and loading

Empty renders `empty` inside the body, spanning every column, with the header still shown.
Loading renders `skeleton` rows at the real row height, so the table does not jump.

## Accessibility

It is a table with a header row. Header cells report their sort direction. The body is one
tab stop; arrows move within it. Fine detail is in [anti-patterns.md](../anti-patterns.md).

## Not normative

Column resizing, column reordering, and virtualisation. A platform may offer them. None is
required, and a table without them is complete.
