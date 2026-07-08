---
name: forge-design
description: Builds SolidJS UIs with the Forge design system — a dark-default, dense, technical-tools aesthetic for dashboards, consoles, observability and admin panels. Ships the full colour scheme/design tokens (CSS variables), component CSS, and SolidJS primitives (Button, Card, Badge, Input, Stat, Toast, tables, logs, app shell). Responsive out of the box — the shell collapses to an off-canvas drawer on narrow screens and touch targets grow under coarse pointers. Use when building or styling SolidJS components or pages, choosing colours for a new component, wiring design tokens into a project, or when the user says "use the design system", "Forge style", "match the console look", "make this look like the tech tools design", "make it work on mobile".
user-invocable: true
argument-hint: [what to build or style]
---

<overview>
Applies the **Forge design system** (synced from the claude.ai design project "Tech Tools
Design System") to SolidJS work. It provides copy-in assets — the token CSS (colours, type,
spacing, radii, motion), the component CSS (`.fbtn`, `.fcard`, `.ftable`, …), and a SolidJS
port of the UI primitives — plus reference docs on the colour scheme and the rules for
designing new components that fit the system. Output is production SolidJS code or styled
components that render dark by default and honor `prefers-color-scheme`.
</overview>

<variables>
- `${CLAUDE_SKILL_DIR}`: Path to this skill's directory.
- `$ARGUMENTS`: What the user wants built or styled (may be empty — then ask).
</variables>

<workflow>
<step order="1">
If `$ARGUMENTS` is empty and no UI task is in context, ask the user what they want to
build or style before doing anything else.
</step>

<step order="2">
Read `${CLAUDE_SKILL_DIR}/reference/solidjs.md` (setup, Solid gotchas, primitives, screen
patterns). If the task involves picking colours, creating a **new** component, or anything
not covered by the existing primitives, also read `${CLAUDE_SKILL_DIR}/reference/tokens.md`
and follow its new-component checklist.
</step>

<step order="3">
Wire the assets into the target project as described in `reference/solidjs.md`: copy
`assets/colors_and_type.css`, `assets/console.css`, and `assets/ui.jsx` in, import the two
CSS files at the app entry (tokens first). Copy — never symlink into this skill directory.
For each asset that already exists in the project, run `diff -u <project copy> <skill copy>`:
identical → leave it and continue; different → show the diff and ask the user whether to
update the project copy. Never overwrite an existing project copy without approval.
</step>

<step order="4">
Build the UI. Reuse the primitives and `console.css` classes before writing new CSS; when
new CSS is unavoidable, every colour/size/duration must be a `var(--token)` reference —
no hardcoded hex, oklch, px-durations or shadows. Put new files where the "Where new code
goes" section of `reference/solidjs.md` says — never append to the copied Forge assets.
</step>

<step order="5">
Validate: render the app in both themes (toggle `data-theme` on `<html>`) and at 375px and
768px viewport widths — the drawer opens/closes from the hamburger and nothing forces
page-level horizontal scroll. Run any new component through the checklist at the end of
`reference/tokens.md`. Fix violations before presenting the result.
</step>
</workflow>

<boundaries>
<always>
- Default to dark theme and honor `prefers-color-scheme` — never light-only
- Use CSS variables from `colors_and_type.css` for every colour, size and duration
- Use SolidJS idioms: `class`/`classList`, `splitProps`/`mergeProps`, `Show`/`For` — never destructure props
- Use Lucide icons (`lucide-solid`) at 1.5px stroke, `currentColor`
- Keep density: 32px controls/rows, 14px body, sentence case, tabular numerals with units
- Keep desktop density — touch sizing applies only under `pointer: coarse`, never by viewport width
</always>

<ask>
- What to build, if invoked with no arguments and no UI task in context
- Before adding any runtime dependency beyond `lucide-solid`
- Before overwriting Forge asset files that already exist in the target project
</ask>

<never>
- Hardcode colours that exist as tokens, or invent new colours outside the palette rules in `reference/tokens.md`
- Use drop shadows, gradients, frosted-glass cards, emoji, or unicode-as-icon (`→`, `✓`)
- Put the accent colour on large background fills or pill-shaped buttons
- Hide content on narrow screens beyond the documented reflows (breadcrumbs ≤768px) — reflow, don't remove
- Edit the files in `${CLAUDE_SKILL_DIR}/assets/` — they mirror the design project (id
  `019dc74c-a1ff-74d0-8504-0ad85b5589fe`); re-sync from there via the DesignSync tool instead
</never>
</boundaries>
