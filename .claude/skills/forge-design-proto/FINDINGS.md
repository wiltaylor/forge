# What the recast exposed

Findings from re-casting `button`, `combobox` and `table` — twelve pages — against the
#66 template. These are the defects the template surfaced, not decisions it settled.

## 1. `laws.md` has no geometry law, and the recast dropped Forge's radius on the floor

`button`'s #64 page said "32px high, 4px radius, 12px horizontal padding" in its Anatomy
prose. Under "a Contract never restates a law", each claim had to go to `laws.md` or into
the Contract.

Only the height was there. `laws.md` fixes control height and body text size and says
nothing about radius or padding. **4px radius appears nowhere normative in the whole
catalogue** — its only mention is inside an `anti-patterns.md` *fix* line ("The pill
button. Fix: 4px radius, 32px high"), which is guidance for repairing a failure, not a
statement of the rule. 12px horizontal padding survives only inside the egui
implementation page, as a number in a paint instruction.

So the recast made a real hole visible rather than creating one: these are universal
geometry, they belong in a `laws.md` geometry section, and today an agent that never hits
the pill-button anti-pattern is never told the radius. Content is #68's extraction job;
the hole is recorded here.

## 2. The generated gap statuses were wrong, and #64's index column shares the method

Seeding `gaps.md` by matching control names against filenames in `crates/forge-egui` and
`crates/forge-tui` reported **29 gaps on egui and 33 on ratatui**. Three spot-checks by
grep killed it:

- `diff-editor` — present on both. Four files each.
- `calendar` — present on both.
- `chat-prompt` — present, at `widgets/specialty/chat/prompt.rs`.

The controls live inside shared modules, so a per-control filename never appears.
`app-shell` is `runtime/shell.rs`. The register on this branch therefore carries **three
rows, each confirmed by grep** — `code-editor` on egui and ratatui, `node-graph` on
ratatui — and says outright that it is not an audit.

This matters beyond the register. #64's per-platform index status column was generated the
same way, so **every `complete` it asserted is unverified in exactly this manner**. Direct
input to [Audit the control-by-platform grid](https://github.com/wiltaylor/forge/issues/72),
which should treat the whole thing as unseeded.

## 3. The combobox filter divergence now has a field, and still has no owner

Both Rust pages carry a `## Contract defects` section against the same Contract line:
egui ranks prefix matches first, ratatui scores a fuzzy subsequence, the Contract says a
plain case-insensitive substring. The template does its job — the defect is now recorded
in a named field on the page that carries it, cross-linked to the other offender.

But recording is not resolving. Somebody has to decide whether substring survives as the
Contract or whether the Contract changes for all three platforms, and **no ticket owns
that decision**. #66 was scoped to deliver the field, and it has.

## 4. Two things stopped looking like departures

Naming platform discretion explicitly turned two apparent divergences into ordinary
statements:

- **ratatui has no `sm` button size.** The #64 page had to explain itself ("asking for one
  is a no-op rather than an error"). The control page now names size as discretion, so the
  implementation page states the fact and moves on.
- **ratatui refuses the popup flip.** The old page called flipping "not normative", which
  reads as permission to ignore. The new page declines it with a reason — in a terminal
  the layout picks the field position, so flipping makes the control jump between frames.

The old "Not normative" heading could only *release* a rule. "Platform discretion" also
lets a platform decline something and say why.

## 5. The Mechanism field split #64's hand-written blockquote cleanly

`egui/combobox` carried one "Divergence, unresolved" blockquote covering two unrelated
things. The template separates them without argument: the hint-text recovery of the
unset/empty distinction is **Mechanism** and always allowed; the prefix ranking is a
**Contract defect**. The same split worked on ratatui, where writing the committed label
into the composed input buffer is mechanism, not departure.

## 6. Cost

The shipped tree went **38,644 → 32,717 tokens, down 15%**. Stripping 219 `status:` lines
and replacing three index tables with link lists more than paid for the extra headings on
the twelve recast pages, which individually grew 1–15%.

`SKILL.md` is untouched at 1,251 tokens. The always-read pair is 1,732.
