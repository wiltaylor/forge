# PROTOTYPE — throwaway

Branch `prototype/page-templates`, built for
[The control-page and implementation-page templates](https://github.com/wiltaylor/forge/issues/66),
a child of [Map: Forge as skills, not a library](https://github.com/wiltaylor/forge/issues/61).

Forked from `prototype/skill-routing`, which answered #64. **Do not merge any of these to
`main`.**

## What this branch adds

- `TEMPLATES.md` — the two templates and the rule that makes them work. Does not ship.
- `FINDINGS.md` — what the recast exposed. Three items need owners.
- `button`, `combobox`, `table` re-cast against the template — 3 control pages and 9
  implementation pages.
- `reference/gaps.md` — the register. Three rows, each confirmed by grep.
- Three gap pages written in the gap form.
- Platform indexes reduced to link lists; `status:` stripped from 219 implementation pages.

## What is still #64's, unchanged

`SKILL.md`, `laws.md`, `anti-patterns.md`, and all 64 untouched control stubs. The
`tokens.md` and `classes.md` stubs are still empty — they belong to
[Where the token and class vocabulary lives](https://github.com/wiltaylor/forge/issues/73).

## What is still not real here

- The `description`. Owned by
  [The forge-design description under the 1,536-char cap](https://github.com/wiltaylor/forge/issues/71).
- The control list and every gap status. Provisional —
  [Audit the control-by-platform grid](https://github.com/wiltaylor/forge/issues/72), and
  read `FINDINGS.md` first: the generated statuses were wrong.
- All 64 stub bodies. Extraction is
  [Extraction plan and the deletion gate](https://github.com/wiltaylor/forge/issues/68).
