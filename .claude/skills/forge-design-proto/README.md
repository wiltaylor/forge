# PROTOTYPE — throwaway

Built for [SKILL.md routing into the catalogue tree](https://github.com/wiltaylor/forge/issues/64),
a child of [Map: Forge as skills, not a library](https://github.com/wiltaylor/forge/issues/61).

**Do not use this for real work, and do not merge it to `main`.** It lives on the branch
`prototype/skill-routing` and exists to answer one question: can `SKILL.md` route an agent
into a ~312-file tree without listing the tree?

## What is real here

- `SKILL.md` — real, written to the three decisions taken on the ticket.
- `reference/laws.md`, `reference/anti-patterns.md` — realistic size, sketch content.
- `button`, `combobox`, `table` — control page plus all three implementation pages, written
  from the actual `packages/` and `crates/` source.
- The tree — 76 control pages, 228 implementation pages, 3 platform indexes. Everything
  outside the three filled controls is a stub with a real path and a status marker.

## What is not real here

- The `description`. Owned by
  [The forge-design description under the 1,536-char cap](https://github.com/wiltaylor/forge/issues/71).
- The page fields. Owned by
  [The control-page and implementation-page templates](https://github.com/wiltaylor/forge/issues/66).
- The control list and the gap statuses. Provisional, from export names and a filename
  match. Owned by
  [Audit the control-by-platform grid](https://github.com/wiltaylor/forge/issues/72).
- All stub bodies. Extraction is
  [Extraction plan and the deletion gate](https://github.com/wiltaylor/forge/issues/68).
