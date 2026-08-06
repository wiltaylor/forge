# Page templates — PROTOTYPE

Built for [The control-page and implementation-page templates](https://github.com/wiltaylor/forge/issues/66),
a child of [Map: Forge as skills, not a library](https://github.com/wiltaylor/forge/issues/61).

This file is the template itself. It does not ship — a real skill carries the pages, not
a description of them. It exists here so the recast pages on this branch can be checked
against something.

## The rule that makes the rest work

**A control page's `## Contract` is the only binding text in the catalogue outside
`laws.md`.** Nothing binding may be stated anywhere else on a control page. Every other
section orients the reader and binds nothing.

This is not a style preference. #64 wrote `combobox` with "filtering is a case-insensitive
substring match on the label" buried in its Anatomy prose. It was binding, it did not look
binding, and both Rust implementations walked past it and shipped a different filter. A
claim that binds has to live somewhere a reader checks.

Two consequences:

- **A Contract never restates a law.** `laws.md` is always read and costs its tokens on
  every compaction regardless. A control page holds only what is specific to that control.
  It may add to a law. It may never contradict one.
- **An implementation page that contradicts the Contract is wrong.** Not a permitted
  variation — a defect in that page, recorded on it under `## Contract defects` until
  someone fixes it.

Reaching a Contract behaviour by a different route is **not** a contradiction. That is
`## Mechanism`, and it is always allowed.

---

## Control page

One per control. Platform-agnostic. Lives at `reference/controls/<control>.md`.

```markdown
# <control>

Implementations: [solid](../impl/solid/<control>.md) ·
[ratatui](../impl/ratatui/<control>.md) · [egui](../impl/egui/<control>.md)

## What it is for

Orientation. Binds nothing. What the control is, and which sibling to reach for
instead — the neighbours are what stop an agent building the wrong control.

## Anatomy

Orientation. Binds nothing. The parts, named, so the rest of the page and every
implementation page can refer to them. No geometry that `laws.md` already fixes.

## State                          <- omit when the control holds none

Orientation. Binds nothing. The shape of the state and what each piece means.
Any rule that must hold about it goes in the Contract, not here.

## Contract

The only binding text on this page. One claim per line, each one checkable against
an implementation page by reading. Specific to this control — never a restatement
of `laws.md`.

## Accessibility

Exactly one line, three slots: the role, who owns the keys, who owns focus.
The cap is the test. A rule that does not fit those three slots is fine detail,
and fine detail is a "do not" in `anti-patterns.md` — never a "do" here.

## Platform discretion

What an implementation may settle for itself. Naming it here is what stops a
platform reading silence as permission.
```

### Why the Accessibility line is capped

#65 measured detailed accessibility prose as **flat-to-worse than a one-liner**: more
rules produce more wrongly-applied attributes. Detailed prose reliably fixes coarse
omissions and unreliably fixes fine ones. The one-liner is the form that measured best,
so the template enforces it by shape rather than by asking an author to stay brief.

The author's test is the cap itself. Role, keys, focus. If a rule needs a fourth clause,
it is fine detail and belongs in `anti-patterns.md` as a named failure.

---

## Implementation page

One per `(control × platform)`. Lives at `reference/impl/<platform>/<control>.md`.

Pages range about 10x in length, and no rule decides which controls deserve length. So
the template fixes three slots and prescribes nothing after them.

```markdown
# <control> — <Platform>

control page: [<control>](../../controls/<control>.md)

## Shape

What you actually type. The class tree on SolidJS; the widget and paint structure
on egui and ratatui. This is the one thing an agent cannot derive.

## What <platform> gives you

What the platform supplies free, and therefore what this page does not cover.
This field explains the page's length instead of leaving it mysterious — the
SolidJS combobox is short because the browser supplies focus, tab order,
scroll-into-view and ARIA, and the egui page is 3x longer because egui supplies
none of them.

## Mechanism                      <- omit when the platform reaches the Contract directly

Where this platform reaches a Contract behaviour by a different route. Same
behaviour, different means. Always allowed, and never a defect. Say what NOT to
do to look more like another platform — the wrong move here is usually to import
the other platform's shape.

## Contract defects               <- omit when there are none, which is the normal case

A behaviour on this page that contradicts the control page's Contract. This is an
error in this page, not a permitted variation. State which Contract line, what
this page does instead, and that it needs fixing.

<free-form headings from here, in whatever order the build wants>
```

### Gap and not-possible pages

A gap page's **body is the notice**. It asserts no status field, so there is nothing to
drift from `gaps.md`. #64 settled zero-hop direct addressing — an agent goes straight to
`impl/egui/markdown.md` without passing an index — so a missing page cannot be silently
empty or the reader invents.

```markdown
# <control> — <Platform>

control page: [<control>](../../controls/<control>.md)

**GAP — not built on <platform>.** Nearest reference: <the platform whose page is
closest, and why>. Fill it by building this control in a real target app from the
control page, then write this page from that working code.
```

```markdown
# <control> — <Platform>

control page: [<control>](../../controls/<control>.md)

**NOT POSSIBLE on <platform>.** <The reason, in one or two sentences.>
Use <substitute> instead.
```

A complete page says nothing about its status. `gaps.md` is the only place a gap is
registered, so 200-odd complete pages carry no field that can go stale.

### Provenance is not a field

A page written from a gap-filling build in a real target app is identical in form to one
extracted from a deleted crate. No marker, no citation, no confidence note. Where a page
came from is git history.

A citation would point at a target app the skill must not depend on, and the Fidelity
constraint already rules that examples are not golden references. The page stands alone
or it is not finished.

---

## Platform index

`reference/impl/<platform>/index.md`. Links only, grouped. It carries no status column,
because `gaps.md` owns every status and two places to write one is two places to drift.

## The gaps register

`reference/gaps.md`. One table, every platform. The single place a status is asserted,
and the direct input to the handoff spec's ticket list.
