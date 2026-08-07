# Does guidance-only actually reproduce a control?

Findings from wayfinder [#67](https://github.com/wiltaylor/forge/issues/67). Six fresh
agents built `button` and `combobox` on SolidJS, ratatui and egui from this tree alone,
each in a scratch project outside the repo with no access to `packages/` or `crates/`.

Method fixed before any result was seen:

- The tree was first patched to #73's settled name shape — `tokens.md` filled by
  mechanical extraction, three grammar pages added, the ten unprefixed screen classes
  spelled inline in `laws.md`. **No geometry law**, so #77 stays open and this is evidence
  for it.
- Two separate scores. **List 1 — page-conformance**: did the agent build what the page
  says? **List 2 — page-versus-source**: where does the page itself disagree with the
  library? List 1 judges the premise; list 2 is a dry run of #68's grounding check.
- **Verdict rule, fixed in advance**: every finding is classified *fixable by amending the
  entry* / *fixable only by shipping code* / *accepted drift*. The premise survives if the
  middle class is empty.
- Every probe was handed a sanitised copy of the skill with every `wayfinder` and
  `PROTOTYPE` marker stripped, so none could tell it was being tested.

## Verdict

**The premise survives.** No finding needs shipped code. The middle class is empty.

Every failure below is a sentence that is missing, ambiguous or wrong on a page — and a
sentence is fixable with a sentence. Nothing required a validator, a generator, a copy-in
asset or a template. #69's guidance-only ruling stands unchallenged by the evidence.

That is not the same as saying the format is cheap. The largest finding is a hole nobody
had noticed, and closing it makes one page considerably bigger.

## The headline: the catalogue names every token and gives the value of none

**All six probes independently invented the entire colour palette.** `tokens.md` as
settled by #73 is a name table — `role | SolidJS | ratatui | egui` — and names are all it
carries. It tells an agent to type `var(--bg-1)` and never says what `--bg-1` is.

On SolidJS that means writing a `:root` block of made-up hex. On both Rust kits it means
writing `Theme::dark()` out of thin air. Every probe did it, and every probe reported it
first in its own list of things it had to guess.

The results were closer than expected and still wrong:

| Token | Probe (solid-button) | Real |
|---|---|---|
| `--bg-0` | `#0b0d10` | `#0B0D10` — exact |
| `--bg-1` | `#12151a` | `#11141A` |
| `--border` | `#262c34` | `#262C36` |
| `--accent` | `#4c8dff` | `oklch(0.62 0.16 250)` — visibly brighter and more saturated |
| `--accent-contrast` | `#06090f` | `#FFFFFF` — **inverted** |

The neutrals converge because "very dark blue-grey" is a small space. The accent does not,
and `--accent-contrast` came out backwards: the probe put near-black text on the accent
solid where Forge puts white. That is a visible, wrong primary button.

The exact `--bg-0` match is worth naming as a caveat: isolation was honour-system, so a
peek cannot be ruled out by evidence. It reads as convergence rather than copying —
`--bg-1`, `--bg-2` and `--border` are all near-misses, and no copier reproduces
`--accent-contrast` backwards.

**Fix: `tokens.md` carries the values, not just the names.** Both ramps, all three
platforms. Hex is prose; this stays inside the guidance-only constraint. It is the single
biggest amendment this probe demands, and it costs a page that is already on-demand.

### The same hole, one level down

Two probes hit a second version of it independently. The status triple is
`{base, bg, fg}`, where `fg` is text on the 14% *tint*. **Nothing names text on a solid
status fill.** Both Rust probes reused `accent.contrast` on the danger solid and both
flagged it as a real gap.

The library has the identical hole and papers over it with a literal —
`.fbtn-danger { color: #fff; }`. So this is not a transcription miss; it is a genuine gap
in the token set that the catalogue inherits.

## List 1 — page-conformance

### `combobox` passes, and passes well

The SolidJS combobox probe is the strongest result in the run. Rendered beside the
gallery it is near-indistinguishable: search glyph, chevron, popup below the field, active
row on the raised surface, label left and control right in the settings row. Every class
it emitted is from the Shape block; it minted none.

Its behaviour matches all eleven Contract lines, verified in a browser: unset at open,
typing filters and sets `active` 0, Enter commits and clears the query, Escape restores
the label and keeps the value, disabled rows are no-ops that leave the popup open, arrows
saturate without wrapping, click-away behaves as Escape.

This is the ticket's central bet, met. A hard control, described in prose, rebuilt
faithfully by an agent that had never seen the code.

### Four entry defects the probes exposed

**1. The `loading` no-resize guarantee is conditional, and the Contract does not say so.**

> `loading` shows a spinner in place of the leading icon and takes the disabled path. The
> label stays mounted, so the button does not resize.

Two probes, on two platforms, reported the same thing: a button with **no** leading icon
has no slot for the spinner to replace, so it grows. The SolidJS build visibly widens
between idle and in-flight. The egui probe pinned the behaviour in a test and flagged it.

Fix: say the guarantee holds when a leading icon is present, and say what happens when it
is not.

**2. "the popup holds a single empty line" is ambiguous, and one probe read it literally.**

The ratatui probe defaulted its no-match text to `""`, so a caller who does not pass one
gets a **blank popup** — the no-match state is invisible. It is a defensible reading of
"empty line". The SolidJS probe read it the other way and made the text a required prop.
The library ships `'No matches'` as a default.

Fix: state that the line carries text and that the default is supplied, not required.

**3. A control cannot be reopened by mouse after committing.**

> Focusing the field opens the popup.

After Enter the field still holds focus, so a click fires no focus event and there is no
route back to the popup. The SolidJS probe found this **by testing, not by reading**, and
added an open-on-click path.

The real library has the identical bug — `onFocus` is its only open path. So this is a
Contract that is incomplete and a library that is wrong in the same place.

**4. The ratatui `default` variant as written is unreadable.**

> Default is the border colour as a foreground on the surface.

The ratatui probe implemented it literally and reported that a border role is a hairline
value, so the label lands near 2:1 against any plausible surface — an enabled Default
button reads dimmer than a disabled one. It declined to tune its hexes to hide it.

The web Contract puts the border role on the 1px stroke and keeps the label at `text`. The
ratatui page over-compressed when it dropped the stroke.

### Two findings that are prototype artefacts, not format failures

Every probe reported that pages it was routed to say "Not written". That is true of **216
of 231** implementation pages and **73 of 76** control pages in this tree — only `button`,
`combobox` and `table` were ever filled. In the shipped catalogue this does not arise.

The real finding underneath is smaller and does survive: **the `button` Contract requires a
spinner, so `button` cannot be built without `spinner`.** Both button probes had to invent
one. In a full catalogue that is a link away and costs reading, which #64 already measured
— but it is control composition, which #61's fog already records as having no home.

## List 2 — where the pages disagree with the library

A dry run of #68's grounding check, on six cells. It found plenty.

### `button` — the page describes a control that does not exist

| Page claim | The library |
|---|---|
| Contract: `loading` shows a spinner and takes the disabled path | **No `loading` state on any of the three platforms.** Invented outright |
| Variants are `primary` / `default` / `ghost` / `danger` | SolidJS ships **`secondary`**, not `default`. The page's name emits `.fbtn-default`, which has no rule — an unstyled button |
| "Sizes are `sm` and the default. There is no large." | `.fbtn-lg` is in the stylesheet and `ControlSize` is `sm \| md \| lg`. **The gallery displays a Large button** |
| Anatomy: "optionally an icon after it" | `ButtonProps` has one `icon` prop, leading only |
| ratatui: focus is "a reversed style plus a `>` gutter marker" | `BOLD \| UNDERLINED` plus a background step. No reverse, no marker |
| ratatui: "a one-row control has nowhere to put a border" | The widget draws a bordered block at 3+ rows |
| ratatui: "`handle_key` returns `Submitted` on Enter or Space" | `Button` has no `handle_key` — "activation is the caller's job" |
| egui: "`disabled` uses `ui.add_enabled_ui`" | It uses `Sense::hover()` |
| egui: default variant takes "the raised surface" | It fills `bg[2]`; `bg[1]` is the raised surface |

The ratatui divergence is measurable rather than arguable. Rendered through `TestBackend`:

```
real forge-tui:    |  Deploy      |   BOLD, centred, no gutter marker
probe (per page):  |>  Deploy     |   REVERSED, `>` gutter marker
```

The probe did exactly what the page said. The page does not describe the library.

### `combobox` — the page is nearly true

Every Contract line checks out against `packages/ui/src/forms.tsx`, classes included. Both
recorded Rust Contract defects are real, not misreadings: egui ranks prefix matches first,
ratatui scores a fuzzy subsequence, and the Contract says plain substring.

One false claim: the SolidJS page says the browser supplies "scroll-into-view for the
active option". **It does not**, for a `div` list — the probe found this by testing. The
library does not implement it either, so the page is wrong and the library is incomplete
in the same place.

### Geometry is inconsistent in the source itself — direct input to #77

`#77` asks whether `laws.md` gains a geometry section and what binds in it. The source
cannot currently support a radius law:

- SolidJS `.fbtn` hardcodes `border-radius: 4px` — a literal, not `var(--r-sm)`.
- egui paints `t.radius.md`, which is **6px**.
- ratatui has no radius at all.

Two platforms, two different radii, for the same control. A geometry law would be
contradicted by the code it was extracted from on the day it was written.

Two more, in the same family: `.fbtn` sets `font-size: 13px` while `laws.md` says body
text is 14px; and `laws.md` says motion is 120ms while the tokens ship 80/160/240 and no
120 exists. Three probes noticed the motion contradiction unprompted and all three
followed the token, citing *the toolkit wins*.

### Two errors this session introduced, and a probe caught both

Both came from the patch written today, which makes them the sharpest available evidence
for what the grounding check is for:

- `ratatui.md` said the crate is **pinned to ratatui 0.29**. That is transcribed from the
  crate's own doc comment. `crates/forge-tui/Cargo.toml` says **0.30**. *A doc comment is
  not a source of truth; the grounding check must read the manifest.*
- `ratatui.md` and `egui.md` gave `ComboBox` as the worked PascalCase example. The real
  type is **`Combobox`** — one word, no internal capital. The ratatui probe spotted the
  collision with the implementation page's Shape block, reasoned that `SKILL.md` routes
  Rust type names to the grammar page, followed the grammar page, and got the wrong name.

The second is the more instructive. **When a name page and an implementation page
disagree, the routing rule decides — and the routing rule pointed at the wrong one.** #73
put the name pages in the grounding check's scope; this is why.

Both are fixed on this branch.

## Classification

| Finding | Class |
|---|---|
| `tokens.md` gives no values | Amend the entry — hex is prose |
| No "text on solid status" token | Amend the entry — and the library has the same gap |
| `loading` no-resize guarantee is conditional | Amend the entry |
| "single empty line" is ambiguous | Amend the entry |
| No mouse route back to the popup | Amend the entry — and fix the library, if it survives |
| ratatui `default` variant is unreadable | Amend the entry |
| Every `button` divergence in list 2 | Amend the entry — transcription, which is #68's job |
| `ComboBox` / ratatui 0.29 | Amend the entry — already done |
| Accent palette will drift per app | Accepted drift, already ruled by #70 |
| Pages say "Not written" | Prototype artefact, not a finding |

**Nothing is in the "needs shipped code" class.**

## What this does not settle

- Isolation was honour-system, by choice. No probe can be proven not to have peeked.
- Two controls of 73, on the two that were recast. `table` was not probed.
- Visual fidelity was judged against an invented palette on every platform, so "does it
  look like Forge" was necessarily scored on layout, density and geometry, not hue.
- egui rendered nothing: the probes built and tested headlessly, but no egui screenshot
  was taken, so its visual axis rests on code and its own `kittest` tests.
