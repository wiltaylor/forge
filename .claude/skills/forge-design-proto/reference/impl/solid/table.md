# table — SolidJS

<!-- PROTOTYPE (wayfinder #64). -->

status: complete · control page: [table](../../controls/table.md) ·
index: [solid](index.md)

```
div.ftable-wrap                 the scroll container
  table.ftable
    thead > tr > th.ftable-h    .is-sorted-asc .is-sorted-desc .is-num
    tbody > tr.ftable-row       .is-active .is-selected
      td.ftable-cell            .is-num
```

A real `<table>`. Semantics, header association and the reading order come free.

Sticky headers are `position: sticky; top: 0` on the `th`, not on the `thead` — Safari
ignores it on the section. The scroll container is the wrapper, never the page.

Right-align numeric columns with `.is-num`, which also applies tabular numerals. Do not
right-align in a style attribute; the class carries both.

Truncation is `overflow: hidden; text-overflow: ellipsis; white-space: nowrap` on the cell,
and it needs a `max-width` on the cell to take effect at all. Put the full value in a
`title` so it stays reachable.

The body is one tab stop: `tabindex="0"` on `tbody`, and arrow keys handled there. Rows are
not focusable. Use the shared roving-index primitive rather than tracking the active row by
hand.

Scroll-into-view for the active row uses `scrollIntoView({ block: 'nearest' })` on the row
element. `nearest` matters — the default scrolls the row to the top on every arrow press.

`For` over rows, keyed by the caller's row key. An index key re-renders the whole body on
a sort.

Sorting is requested only. Fire the callback; do not sort a local copy.
