# table

<!-- RECAST against the #66 template. #64's ad-hoc "Column alignment" section, which
     closed with "this is not a preference", was the clearest sign the page had binding
     claims outside its contract. It is now two Contract lines. -->

Implementations: [solid](../impl/solid/table.md) · [ratatui](../impl/ratatui/table.md) ·
[egui](../impl/egui/table.md)

## What it is for

Rows of records with the same shape, compared across columns. `logs` when the rows are a
stream and the newest matters most. `list-box` when there is one column and the point is
selection.

## Anatomy

A header row, then body rows. Optional leading selection column. Optional trailing actions
column.

## State

| State | Meaning |
|---|---|
| `rows` | The data. Caller-owned |
| `sort` | Column and direction, or none |
| `selection` | Selected row keys. Caller-owned |
| `active` | The keyboard row |

## Contract

- Numbers are right-aligned and tabular. Text is left-aligned. Status glyphs are centred.
  A right-aligned number column is how the eye compares magnitudes, so this is not a
  preference.
- The header is the sunken surface with the dim text role, and stays visible while the
  body scrolls.
- Sorting is requested, never performed. The table reports the column and direction the
  user asked for; the caller sorts and passes new `rows` back. A table that sorts its own
  copy desynchronises from the caller's paging.
- Up and Down move `active` by a row and stop at the ends without wrapping. Page Up and
  Page Down move by the visible row count. Home and End move to the first and last row.
- Enter activates the `active` row, if the caller supplied an activation handler.
- Space toggles selection on the `active` row, when selection is on.
- Activating a header cell cycles its sort: ascending, descending, none.
- The active row is always visible — scrolling follows `active`.
- Empty renders `empty` inside the body, spanning every column, with the header still
  shown. Loading renders `skeleton` rows at the real row height, so the table does not
  jump.
- The actions column holds `ghost` icon-buttons only.

## Accessibility

Role `table` with a header row; the body is one tab stop and owns the arrows; header cells
report their sort direction.

## Platform discretion

Column resizing, column reordering, and virtualisation. A platform may offer them. None is
required, and a table without them is complete.
