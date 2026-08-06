# Gaps

<!-- RECAST against the #66 template. This register is SEEDED, not audited — three rows,
     each confirmed by grep. The real audit is #72. See FINDINGS.md on why the generated
     version of this list was thrown away. -->

Every control that is missing on a platform, and every control that cannot exist on one.
This is the only place a status is written. A complete implementation page says nothing
about its status, so there is nothing to drift.

`gap` — not built here. Port the behaviour from the control page, not from another
platform's implementation page.
`not-possible` — cannot exist here. The row gives the reason and the substitute, and so
does the page.

Every row becomes a ticket in the handoff spec.

| control | platform | status | why |
|---|---|---|---|
| code-editor | egui | gap | not built |
| code-editor | ratatui | gap | not built |
| node-graph | ratatui | gap | not built |

**No confirmed `not-possible` cell yet.** The row form is:

| control | platform | status | why |
|---|---|---|---|
| *example* | *ratatui* | *not-possible* | *reason, in one clause. Use `<substitute>`.* |

**This table is three verified rows, not an audit.**
[Audit the control-by-platform grid](https://github.com/wiltaylor/forge/issues/72) owns
the real one. Do not read an absent row as "complete".
