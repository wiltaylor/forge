# Skill-authoring constraints and prior art

Research findings for [#65](https://github.com/wiltaylor/forge/issues/65), a child of the
map [#61](https://github.com/wiltaylor/forge/issues/61) ("Forge as skills, not a library").

**Date:** 2026-08-05 · **Claude Code version inspected:** 2.1.222 · **Branch:** `research/skill-authoring`

> **Where this file lives.** The repo had no existing convention for research notes — `docs/`
> held only two frozen normative specs (`api-contract.md`, `widgets-protocol.md`). This is
> filed under `docs/research/` as a new, clearly-separated home for non-normative notes.

## Source trust tiers

| Tier | Sources used |
|---|---|
| **A — primary, verified here** | Official Claude Code docs (`code.claude.com/docs/en/skills.md`, `sub-agents.md`), fetched and quoted directly. Anthropic's bundled `dataviz` and `design-sync` skills, extracted verbatim from the Claude Code 2.1.222 binary. The four skills in this repo's `.claude/skills/`, measured directly. |
| **B — primary, fetched by a sub-agent** | Vendor docs and engineering blogs (Atlassian, shadcn, MUI, Shopify, 1Password), peer-reviewed papers. The two most decision-relevant (Atlassian DESIGN.md post, google-labs-code DESIGN.md spec) were re-verified directly for this doc. |
| **C — weak** | Self-published preprints, practitioner essays, directory listings. Flagged inline. |

---

# Strand 1 — Mechanics

## 1.1 The hard numbers

Every figure below is from `https://code.claude.com/docs/en/skills.md`, quoted verbatim.

| Constraint | Value | Exact wording |
|---|---|---|
| SKILL.md length | **No hard limit; ~500 lines recommended** | "Keep `SKILL.md` under 500 lines. Move detailed reference material to separate files." |
| `description` (+ `when_to_use`) | **1,536 chars, truncated** | "the combined `description` and `when_to_use` text is truncated at 1,536 characters in the skill listing to reduce context usage" |
| Skill-listing budget | **1% of the model's context window** | "The budget scales at 1% of the model's context window. When the listing overflows, Claude Code drops descriptions starting with the skills you invoke least" |
| Post-compaction survival | **first 5,000 tokens per skill, 25,000 tokens total** | "Claude Code re-attaches the most recent invocation of each skill after the summary, **keeping the first 5,000 tokens of each**. Re-attached skills share a combined budget of 25,000 tokens." |
| `name` | No documented char limit; directory name is what you actually type | |
| Skill count | No documented cap | |

Tunables: `skillListingBudgetFraction` (raise the 1%), `skillListingMaxDescChars` (raise the
1,536), `SLASH_COMMAND_TOOL_CHAR_BUDGET`. `/doctor` estimates the listing's cost;
`/context` reports the post-budget size.

### The compaction ceiling is the real size limit, and nobody talks about it

The 500-line tip is advisory. **The 5,000-token-per-skill compaction budget is not.** A
skill whose body exceeds ~5k tokens is silently truncated at its head the first time the
session compacts, and the tail — which in a control catalogue is where most of the controls
live — is gone. With several skills invoked, the 25k combined budget starts dropping older
skills entirely.

Measured for scale: this repo's `packages/` is 17,717 lines of TS/TSX/CSS. Atlassian's
50+-component DESIGN.md is 80 KB ≈ 19,800 tokens — **~4× over the per-skill compaction
budget**, from a system smaller than Forge's ~60 controls.

**Implication for the map:** a "full control catalogue" cannot live in one SKILL.md body.
It has to live in `references/` files that are read on demand, and even then, once read,
those file contents are ordinary conversation turns subject to normal compaction.

## 1.2 Progressive disclosure — what it actually does

- **At session start:** only names + descriptions, subject to the 1% budget.
- **On invocation:** the *rendered SKILL.md* enters the conversation as a single message and
  "stays there for the rest of the session." Claude Code **does not re-read the file on later
  turns** — so guidance must be written as standing instructions, not one-time steps.
- **Re-invocation** with identical rendered content adds a short note, not a second copy
  (v2.1.202+). Differing content (changed args, dynamic context) appends the full body again.
- **Bundled `references/*.md` are NOT auto-loaded.** Claude must open them with the Read
  tool. The mechanism is a plain markdown link relative to the skill dir:
  `- For complete API details, see [reference.md](reference.md)`. `${CLAUDE_SKILL_DIR}` and
  `${CLAUDE_PROJECT_DIR}` substitutions are available in the body.
- No documented depth limit or file-count recommendation for the bundled tree.
- A skill does **not** need `Read` in `allowed-tools` to open its own files — baseline
  permissions govern.

**Consequence:** whether a reference file is read at all is a *prompt-engineering* outcome,
not a mechanism. `writing-great-skills` (local authority) names this exactly:

> A **context pointer**'s _wording_, not its target, decides when and how reliably the agent
> reaches the material. … A must-have target behind a weakly worded pointer is a variance bug.

## 1.3 `description` and triggering

The description is the *only* thing the model sees before deciding. Official guidance:
put the key use case first (because of the 1,536 cap), include "keywords users would
naturally say", make it more specific if it over-triggers. **Nothing in the docs promises
deterministic triggering.**

`writing-great-skills` adds the sharper rule: one trigger per *branch*, collapse synonyms,
cut identity already in the body, and front-load the skill's **leading word** — a
pretrained concept that, if it also lives in your prompts and codebase, makes the skill
fire more reliably.

### Live finding: this repo's `forge-design` description is over the cap

| Skill | `description` chars | Body lines |
|---|---|---|
| **`forge-design`** | **1,751 — truncated at 1,536** | 107 |
| `playpen` | 586 | 126 |
| `forge-tauri` | 418 | 103 |
| `playwright-cli` | 77 | 383 (over the 500-line tip's spirit; all body, no refs used at load) |
| Anthropic `dataviz` | 1,020 | 113 |

`forge-design` loses its last ~215 characters — the tail of its trigger-phrase list
("build a chat", "chat UI", "conversation view", "assistant transcript", "render markdown").
Those triggers do not exist as far as the router is concerned. This is a concrete instance
of the failure the map's decomposition is meant to fix: one skill trying to be the trigger
surface for everything.

## 1.4 Frontmatter — the authoritative field list

Every field is optional; only `description` is recommended.

| Field | Semantics (verbatim or condensed from the docs) |
|---|---|
| `name` | Display label only for personal/project skills — **the command comes from the directory name**. For *plugin* skills it sets the last command segment. |
| `description` | What + when. 1,536-char cap shared with `when_to_use`. |
| `when_to_use` | Trigger phrases; appended to `description`, counts toward the cap. |
| `argument-hint` | Autocomplete hint. |
| `arguments` | Named positional args for `$name` substitution. |
| `disable-model-invocation` | `true` → Claude can't auto-load it; **also blocks preloading into subagents** and scheduled-task firing. Default `false`. |
| `user-invocable` | `false` → hidden from the `/` menu **only**. Default `true`. "The `user-invocable` field only controls menu visibility, not Skill tool access." |
| `allowed-tools` | **A permission grant, not a restriction.** "It does not restrict which tools are available: every tool remains callable." Grant covers only the turn that invoked the skill and clears on the next user message. |
| `disallowed-tools` | The actual restriction — removes tools from the pool while the skill is active; also clears next message. |
| `model`, `effort` | Turn-scoped overrides. |
| `context: fork` | Run the skill in a forked subagent context. |
| `agent` | Which subagent type, when `context: fork`. |
| `background` | With `context: fork`, `false` = block for the result. v2.1.218+. |
| `hooks` | Skill-lifecycle hooks. |
| `paths` | **Glob patterns that gate auto-activation** — the skill only auto-loads when Claude is working with matching files. |
| `shell` | `bash` (default) or `powershell` for inline `` !`cmd` `` blocks. |

Booleans accept `yes/no/on/off/1/0` since v2.1.218.

### The local `meta-skill` frontmatter spec is stale — correct it

`~/.claude/skills/meta-skill/reference/frontmatter-spec.md` and `scripts/skill-lint.py`
disagree with the current docs on five points. Anything the map builds on `meta-skill`
inherits these errors:

| Claim in local spec | Reality |
|---|---|
| `description` max **1024** chars (lint **FAILs** above it) | Cap is **1,536**, and it *truncates* rather than erroring |
| `user-invocable` defaults to **`false`** | Defaults to **`true`** |
| `allowed-tools` "restricts which tools the skill may use" | It **grants permission**; it restricts nothing. `disallowed-tools` is the restriction |
| `context` = "list of files to inject into context" | `context` = `fork`, a subagent-execution switch. There is no file-injection field |
| `agent` = "agent file path relative to `.claude/agents/`" | `agent` = subagent *type* name, paired with `context: fork` |
| `metadata`, `license`, `compatibility` listed as supported | Not in the Claude Code field table |

The lint's `BODY_LINE_BUDGET = 150` is stricter than the docs' 500 — a house style, not a
platform limit. Worth keeping; just know which is which.

## 1.5 Discovery and precedence

| Location | Path |
|---|---|
| Enterprise | managed-settings location |
| Personal | `~/.claude/skills/<name>/SKILL.md` |
| Project | `.claude/skills/<name>/SKILL.md` |
| Plugin | `<plugin>/skills/<name>/SKILL.md` → `/plugin-name:skill-name` |

Precedence, verbatim: **"enterprise overrides personal, and personal overrides project."**
(Note the direction — a personal skill *shadows* a project skill of the same name. That is
the opposite of most tooling and a live hazard for a repo shipping `forge-design` while a
developer has an older personal copy.) Any of these also overrides a bundled skill of the
same name. Plugin skills are namespaced and cannot conflict.

Other mechanics that matter to the map:

- **Nested `.claude/skills/` below the working directory are picked up lazily** when Claude
  reads/edits a file in that subdirectory, and appear as `apps/web:deploy`. Invoking the
  unqualified name appends a list of qualified variants with an instruction to also invoke
  the matching one. This is the closest thing to automatic scoping.
- A skill directory **can be a symlink**; Claude follows it and dedupes if the same target
  is reachable twice.
- Adding `.claude-plugin/plugin.json` to a skill folder makes it load as a plugin named
  `<name>@skills-dir`, letting it bundle agents/hooks/MCP. (Out of scope per the map, but
  it is the only packaging escape hatch that doesn't require a marketplace.)
- `skillOverrides` in settings gives per-skill `"on" | "name-only" | "user-invocable-only" | "off"`.
  `"name-only"` is the budget-relief lever. Plugin skills are unaffected.

## 1.6 Composition and dependency — **the answer is no**

**There is no first-class mechanism for one skill to depend on, import, or require another.**
Checked and ruled out:

- **No `dependencies`/`requires` frontmatter field.** `dependencies` exists only in plugin
  `plugin.json`, for plugin-to-plugin.
- **No `@import` in SKILL.md.** CLAUDE.md has `@path/to/file`; SKILL.md does not. Skills
  reference files only via markdown links the model must Read.
- **No cross-skill file references.** A skill's `${CLAUDE_SKILL_DIR}` is its own directory.
  Nothing resolves a sibling skill's path for you. (You could hard-code
  `.claude/skills/forge-core/references/tokens.md`, but that hard-codes the install layout —
  which the map explicitly forbids by putting distribution out of scope.)
- **Symlinks are directory-level, not file-level** — a *whole skill folder* can be a symlink,
  which is not composition.

What exists instead — three conventions, in descending reliability:

1. **Subagent preload (`skills:` in an agent definition).** Verbatim: *"Skills to preload
   into the subagent's context at startup. **The full skill content is injected, not only the
   description.**"* This is the only *mechanical* way to guarantee two skills' bodies are both
   present. Caveat: *"You can't preload skills that set `disable-model-invocation: true`."*
   It also only works inside a subagent, and costs the full body up front.
2. **`context: fork` + `agent:`** — a skill can run itself in a subagent whose definition
   preloads the core skill. Combining (1) and (2) is the nearest thing to a real dependency.
3. **Prose instruction to invoke the other skill via the Skill tool.** Reliable enough that
   Anthropic's own bundled skills do it — `artifact-design` says *"Load the `dataviz` skill
   for the specifics; this skill continues to govern the page the chart sits in"* — but it is
   a probabilistic pointer, not a guarantee. Note the target must be **model-invocable**
   (i.e. must keep a description, and must not set `disable-model-invocation`), so a
   depended-on core skill pays permanent listing budget.

**Verdict for the map's "Platform skills declare a dependency on the core" constraint:
that dependency is pure convention.** It can be made *mechanical* only by (a) preloading via
a subagent, or (b) collapsing core + platform into one skill with the core material in
shared `references/`. There is no third option.

---

# Strand 1b — How Anthropic's own skills do it

## 2.1 `dataviz` — the pattern the map is aiming at

`dataviz` is **not on disk**; it is a bundled skill compiled into the Claude Code binary
(extracted verbatim from `~/.local/share/claude/versions/2.1.222`). Anatomy:

| File | Lines | Chars | Role |
|---|---:|---:|---|
| `SKILL.md` | 113 | 8,279 | Router + procedure + non-negotiables |
| `references/palette.md` | 193 | 9,360 | The one filled-in instance of every parameter |
| `references/color-formula.md` | 134 | 8,477 | Four jobs, six checks |
| `references/anti-patterns.md` | 119 | 6,336 | Catalogue of what goes wrong |
| `references/marks-and-anatomy.md` | 95 | 6,216 | Exact mark specs |
| `references/interaction.md` | 60 | 3,822 | Tooltips, hover, filters |
| `references/choosing-a-form.md` | 55 | 3,159 | Chart-type heuristic |
| `references/components.md` | 37 | 2,221 | The parts list, tiered |
| `scripts/validate_palette.js` | — | 41,095 | Runnable validator |
| **Total prose** | **806** | **~47.9k (~12k tokens)** | |

`description` is 1,020 chars — comfortably under the cap.

### Five transferable moves

1. **Method / instance separation.** SKILL.md holds a *design-system-agnostic method*; one
   reference file (`palette.md`) is "the reference instance, every value filled in." A table
   in SKILL.md lists the eight parameters a new design system must supply. *This is a working
   answer to the map's core-plus-platform decomposition, and it does it inside one skill.*
2. **The catalogue is a parts list, not a spec-per-part.** `components.md` is 37 lines for
   ~20 chart components — three tiers of one-line entries. The *behavioural precision* lives
   in cross-cutting reference files (`marks-and-anatomy.md`, `interaction.md`) that state
   rules applying to every component, not per-component prose. **This is the opposite of the
   map's "full control catalogue, every one of the ~60 controls documented" constraint.**
3. **Precision is numeric and universal, not per-component.** e.g. bars ≤24px thick, 4px
   rounded data-end square at the baseline; lines 2px round join/cap; markers ≥8px; area
   fills ~10% opacity; gridlines 1px solid never dashed; a 2px surface gap between touching
   marks; a 2px surface ring on overlapping dots. Roughly 20 numbers govern every chart.
4. **The deterministic part is a script, not prose.** *"The single most important habit:
   the color part is computable, so compute it. Never eyeball whether a palette is
   colorblind-safe — run `scripts/validate_palette.js`."* This matches the local
   `authoring-guide.md` rule ("Scripts over prose for deterministic logic") — and it is a
   direct challenge to the map's **guidance-only, no-code** constraint. Anthropic's own
   flagship guidance skill ships 41 KB of executable JavaScript.
5. **A named failure catalogue with a closing check.** *"Then check the result against
   `references/anti-patterns.md` … If your chart matches an entry, it's wrong."* A
   completion criterion, in `writing-great-skills` terms.

The procedure is 7 ordered steps ending in **"Render it and look at it"** — *"The validator
checks color, not layout — open or screenshot the output and eyeball it before calling it
done."* Guidance alone is not trusted to land.

## 2.2 `design-sync` — first-party evidence *against* guidance-only

Also bundled in the binary. Its description:

> "Push a React design system to claude.ai/design. This runs a converter that **bundles the
> real component code** (from Storybook or a bare package) and uploads it."

This is Anthropic's shipped answer to "make my design system usable by an agent", and it is
the opposite of the map's bet. Per component the converter emits: a `.jsx` re-export stub of
**the real compiled component**, a `.d.ts` props interface extracted with ts-morph, a
`.prompt.md` doc, and a rendered `.html` preview card — plus screenshots, a Playwright render
check, a per-component grading loop, and a `guidelines/` folder of copied design-guideline
markdown.

Prose is present, but it is one small layer: a hand-authored `conventions.md`
(**budget: "2–4k characters covers all four concerns"**) prepended to the bundle README and
inlined into the design agent's system prompt. The instructions for writing it are the single
most useful passage found in this research:

> An agent in that position **follows concrete, enumerated guidance and cannot follow guidance
> that isn't there: name the tokens and it uses tokens; leave the class vocabulary unnamed and
> it won't guess at yours — it will invent its own.** Say to wrap in the provider and it wraps;
> don't, and it mostly won't. So every sentence must pass one test: *could the design agent act
> on this without guessing?* ("Follow the design system's conventions" fails that test; delete
> it and write the convention.)

And, on where truth lives:

> Name the stylesheet/source files the agent should read before styling … **An agent that reads
> the real files beats any summary — your job is making sure it knows where to look.**

And a validation rule with teeth:

> **A conventions file that names things which don't exist is worse than none** — the agent will
> trust it, write vocabulary that doesn't resolve, and ship silently unstyled output. Before
> committing: every class, token, prop, and component you enumerated must exist in the built
> artifacts — grep classes/tokens against the compiled stylesheets … Verifies in neither → fix
> the name or cut it.

Also relevant: the header is truncated if it alone exceeds ~31.9k chars, and the generated
README body is cut from the *end* past a ~32k window.

**Read this three ways.** (a) It validates the *style* the map wants — enumerate real names,
never gesture. (b) It is evidence Anthropic does not believe prose replaces components: the
components ship as real code and the prose's job is *routing to them*. (c) The
"name things that don't exist is worse than none" rule is a hard requirement on the map's
extraction order — a catalogue written before deletion can be grepped against the real code;
one written after cannot be verified against anything.

## 2.3 This repo's current skills, measured

| | `forge-design` | `forge-tauri` | `playpen` |
|---|---|---|---|
| SKILL.md body | 107 lines | 103 | 126 |
| `description` | **1,751 (over cap)** | 418 | 586 |
| Reference prose | 640 lines | 118 | 421 + 700 lines of templates |
| Copy-in code | **5,183 lines** of `assets/*.jsx/css` | none | 284 KB incl. a vendored `forge/` copy |

`forge-design`'s reference prose is a **thin API index, not a spec**: `solidjs.md` covers ~30
primitives in ~108 lines (≈3.5 lines each) — enough to *call* a component that already
exists, nowhere near enough to *rebuild* one. `tokens.md` (216 lines) is the genuinely
self-sufficient part and is the closest existing analogue to `dataviz`'s `palette.md`.

**The gap the map must close is roughly an order of magnitude of prose**, and it is precisely
the interaction-behaviour prose that is missing today.

---

# Strand 2 — Prior art

## 3.1 Design systems published as agent guidance: yes, and the shape is consistent

There is a published standard: **Google Labs' DESIGN.md** (Apache 2.0), "a format
specification for describing a visual identity to coding agents" — YAML tokens plus prose
rationale.

Its scope, verified directly against the spec: it "defines the **visual identity** of a brand
and product", covering colour/typography/spacing/radius tokens and "**style guidance** for
component atoms". Components are described through `backgroundColor`, `textColor`,
`typography`, `rounded`, `padding`, `size`, `height`, `width`, with variants for "active,
hover, pressed". The spec does not *forbid* interaction semantics — it simply has no place to
put them. **No keyboard navigation, no ARIA, no focus management, no state machines.** The one
standard for "design system as prose" covers exactly the half of Forge that is easy.

Among vendors, **none ship prose alone**:

| Vendor | Shape |
|---|---|
| shadcn/ui | MCP server browses a registry and installs real components via CLI |
| MUI | `@mui/mcp` — `useMuiDocs` / `fetchDocs` / `generateReactCode`; motivated explicitly by hallucinated props |
| Shopify | `SKILL.md` **plus** `scripts/search_docs.js`, `scripts/validate.js`, `assets/` with compressed schemas |
| Atlassian | MCP server + skills + generated structured content model |
| 1Password | narrow executable skills + MCP "what exists?" server + real product examples |
| Triptease (indie prose skill) | prose for buttons/typography; **instructs the agent to use web components for comboboxes, date pickers, charts** |

The Triptease line is the tell: an independent team writing a prose design-system skill drew
the prose/code boundary at exactly the place the map proposes to cross.

An audit of 37 design systems found MCP server 11/37, `/llms.txt` 10/37, shadcn-spec registry
1/37; 19 systems scored 0/5 on AI-readiness. llms.txt adoption is broad but shallow.

## 3.2 Atlassian ran this experiment and published the numbers

**The closest thing to a controlled test of the map's bet** (verified directly). Atlassian
compressed a mature 50+-component design system into one DESIGN.md and benchmarked it against
their MCP server.

- Full on-demand guidance: **~2.5 MB**. DESIGN.md: **80 KB ≈ 19,800 tokens**.
- To fit, they "remove[d] much of the usage guidance from our 50+ components, heavily
  trim[med] our foundation guidance, and cut a number of design tokens that were low-use."
- Head-to-head on one login-screen task: DESIGN.md **7.21M tokens / 6m46s / 45.3 turns** vs
  MCP **3.75M / 5m01s / 35.1 turns** — **~92% more tokens**.
- Failure mode, verbatim: DESIGN.md "was more likely to **re-create components rather than use
  the existing system**", with greater variance across runs.
- Verdict, verbatim: "DESIGN.md is a useful **portability format** as a snapshot of your design
  system, **not a replacement for richer design system tooling**."

Two caveats on how much this binds Forge. First, "re-create components rather than use the
existing system" is a *drift* complaint in a world where the library still exists — under the
map, re-creating **is** the intended behaviour, so that specific finding partly does not
apply. Second, Atlassian's own guidance corpus is 2.5 MB; Forge's is 17.7k lines. But the
*compression* finding does bind: at 50 components they could not fit usage guidance into a
single portable file, and Forge has ~60 across three platforms.

## 3.3 Does precise prose reproduce a complex interactive control?

**Nobody has published the exact experiment.** No work gives an agent an exact-token,
exact-class-contract spec and measures fidelity across a component library. What exists:

**Interactive correctness is measurably worse than static correctness.**
*Interaction2Code* (ASE 2025; 127 pages, 374 interactions) reports Claude-3.5-Sonnet at
**78.7% implement rate — about one interaction in five not implemented at all** — and an
interaction CLIP score of **0.57 against 0.72 for the full page**. Ten distinct failure types
are catalogued (missing element, no interaction, wrong element/type/position, wrong effect,
partial implementation). This is screenshot-to-code rather than spec-to-code, so it transfers
by analogy, not directly.

**More precise prose is not reliably better than a one-liner — three independent findings.**

- *WebAccessBench* (150 tasks incl. dialogs, 19 models) tested exactly the map's three
  conditions — unguided, "make it accessible", and an expert prompt spelling out
  `<dialog>`, focus landing on the first child, close button last in focus order, focus
  trapped, focus returned on close. Mean errors: **3.22 unguided → 2.01 light → ~2.16
  expert.** Conclusion: "Lightweight guidance is consistently beneficial, while expert-style
  guidance is beneficial only for specific models." ⚠️ **Tier C** — self-published,
  non-peer-reviewed, automated (axe-style) scoring only, and its model ranking puts
  `gpt-5-nano` first and `claude-opus-4.6` seventeenth, which suggests the metric rewards
  small simple DOM. Treat the *direction* as signal, the absolutes as unreliable.
- *W4A 2025* (peer-reviewed; 80 generated UIs): under **automated** testing the
  accessibility-*oriented* prompt scored slightly **worse** (17.32% vs 15.93% violation rate),
  with Non-text content down 83.3% and Label-in-Name down 23.0%.
- *arXiv 2503.15885*: "generic accessibility instructions often lead to **over-application of
  accessibility attributes in inappropriate contexts, creating new issues**." Few-shot
  prompting was the worst condition tested.

The converging mechanism: **more rules → more attributes applied → more wrongly-applied
attributes.**

**Counter-evidence, and it is real.** The same W4A paper's *expert human* evaluation found the
detailed prompt cut the overall violation rate **58% → 19%**, keyboard-accessibility violations
**80% → 0%**, and mean severity **−80.4%**, with residual issues "implementation details rather
than fundamental oversights". And LLMs beat human developers on baseline accessibility on real
projects (AChecker inaccessibility 0.347 GPT-4o vs 0.425 human; contrast issues −49%, alt-text
−70%) — ARIA remaining the standout weakness. Note the task was a bank homepage, not a
combobox.

The honest reading of automated-says-worse / experts-say-much-better: **detailed prose
reliably fixes coarse omissions (no keyboard handler at all, no landmarks) and does not
reliably fix fine ones (which ARIA attribute, on which node, with which id).**

## 3.4 Cross-component consistency

The thinnest evidence area — **no study builds N components from one spec and measures
inter-component coherence.**

- **1Password** ran an agent pipeline over their Knox design system. First attempt was "a
  semi-hot mess": the agent "placed tokens at the wrong tier in the hierarchy. Reached for raw
  HTML elements instead of the correct component primitives." The fix was three layers —
  narrow executable skills committed beside the code, an MCP server so agents could ask "what
  exists?", and real product examples as anchors. Prose docs alone were insufficient. (Tier C:
  company writeup, no numbers.)
- **Atlassian** measured the drift: greater run-to-run variance, and single-file loading meant
  "context truncation occurs in fewer turns."
- **The undocumented-convention problem** (Tier C, practitioner essay, but the sharpest framing
  found): a team deliberately focuses the *cancel* button in destructive dialogs; it was never
  written down; the agent defaults to *confirm*, and the result "looks like yours, imports your
  component, gets through review without a second look." — *"An agent will never hand you back
  an empty gap. It fills the silence with the rest of the world."* This is the exact risk the
  map's "Fidelity — skills must be self-sufficient" constraint is betting it can write away,
  and the extraction-before-deletion ordering is the only defence.

---

# What this changes for the map's standing constraints

## Constraint: **Guidance only — skills ship no code**

**Challenged from three directions.**

1. `dataviz`, the local exemplar the map explicitly points at, is *not* guidance-only — it
   ships a 41 KB executable validator, because the colour rules are deterministic and prose
   loses them.
2. Anthropic's skill-authoring guidance says fragile + error-prone + consistency-critical work
   belongs in bundled deterministic artifacts, not prose.
3. Every vendor shipping "design system for agents" pairs prose with a registry, MCP,
   validators, or real primitives; two of them (Atlassian, 1Password) tried prose first and
   published that it wasn't enough.

**But the constraint as written is narrower than it looks.** It forbids *copy-in assets,
generators, and templates that emit source* — i.e. it forbids shipping the components. It does
not obviously forbid shipping a **checker**. The distinction that survives the evidence:
*prose produces the code; a script verifies the code.* A `validate-tokens.js` that greps the
target repo's CSS for hardcoded hex, or an axe/`eslint-plugin-jsx-a11y` gate, is not a library
and does not drift, because it asserts against the spec rather than embodying it.

**Recommendation:** keep guidance-only for *production*, and carve out an explicit exception
for *verification* artifacts. Otherwise the map is strictly stricter than the exemplar it
cites. Independently: every generative step wants a validation step (local `authoring-guide.md`
rule 4), and `dataviz` ends on "render it and look at it".

## Constraint: **Full control catalogue — every one of ~60 controls documented per platform**

**This is the constraint most at risk, on two grounds.**

*Mechanical.* A body over ~5,000 tokens is truncated at its head on the first compaction; the
combined re-attach budget is 25,000 tokens. Atlassian needed ~19,800 tokens for 50 components
*after* cutting most usage guidance. Sixty controls × three platforms × interaction-level
precision cannot be a monolith — the catalogue must be one reference file per control (or per
tight group), reached by pointer, and even then the reliability of the pointer is prose, not
mechanism.

*Structural.* `dataviz` deliberately does **not** do per-component specs. Its component list is
37 lines of one-liners; the precision lives in ~20 universal numbers plus cross-cutting rules
in `marks-and-anatomy.md` / `interaction.md` / `anti-patterns.md`. That is a materially
different bet: **rules that bind every control** rather than **a spec per control**. It is also
cheaper to keep true.

**Recommendation:** don't discard the full catalogue — for the ~10 hardest controls it is the
whole value. But restructure it as *cross-cutting interaction laws + a per-control file only
where behaviour is non-derivable*. Expect the split to be roughly the one the evidence draws:
Badge/Stat/Alert/Skeleton/Progress/Avatar and layout are handled by rules; Combobox, ListBox,
Select, DatePicker/Calendar, Modal, Dropdown/ContextMenu, Command palette, Table need their own
files. And treat `anti-patterns.md` as a first-class deliverable — it is the closing
completion criterion.

## Constraint: **Core + per-platform skills, platform declares a dependency on core**

**The dependency does not exist as a mechanism.** No `dependencies` field, no `@import`, no
cross-skill file reference. Convention only. Three ways to make it real, in descending
guarantee:

1. **Subagent preload** — an agent definition with `skills: [forge-core, forge-solid]` injects
   both full bodies at startup. The only mechanical guarantee. Requires the core skill to *not*
   set `disable-model-invocation: true`, and costs both bodies up front. Combine with
   `context: fork` + `agent:` so `/forge-solid` runs in that agent.
2. **Collapse** — one skill, core material in shared `references/`, platform material in
   sibling references. `dataviz` does this (method + parameter table in SKILL.md, one filled-in
   instance in `palette.md`). Zero dependency risk, but one description carries every trigger —
   and `forge-design`'s current 1,751-char description is already proof that one skill cannot
   be the trigger surface for everything.
3. **Prose pointer** — "Read the `forge-core` skill first." Anthropic's own bundled skills do
   this (`artifact-design` → `dataviz`), so it's blessed, but it's probabilistic.

**Recommendation:** state in the spec that the dependency is convention, and pick the
enforcement explicitly. Option 1 is the only one that survives a weak model. Whichever is
chosen, the core skill must stay model-invocable (so it keeps a description, and pays listing
budget permanently) or nothing but the human can reach it — per `writing-great-skills`:
*"Because it has no description, nothing but the human can reach it: no other skill can fire it."*

## Two additions the map does not currently carry

**A grounding rule.** From `design-sync`: *"A conventions file that names things which don't
exist is worse than none."* Every class, token, prop and component named in the catalogue must
be greppable against the real source **at the moment it is written**. This turns the map's
"extract from code first, delete after" ordering from a convenience into a correctness
requirement, and it argues for a mechanical check run before deletion is allowed.

**A cheap decisive experiment, before committing.** Nobody has published this, so run it: write
the exact prose spec for one hard control — Combobox with typeahead and roving focus — and have
three fresh sessions build it from that spec alone. Diff the three against each other and
against the APG conformance checklist (role placement, `aria-controls`, `aria-expanded`
toggling, `aria-activedescendant` matching a rendered option id, DOM focus never leaving the
combobox, scroll-into-view, Escape/Enter). Three-way divergence answers both the fidelity and
the consistency question in an afternoon, and tells the map whether the ~10 hard controls need
a different treatment from the other 50. Prototype ticket #66 is the natural home.

---

## Sources

**Primary, verified for this doc**
- https://code.claude.com/docs/en/skills.md
- https://code.claude.com/docs/en/sub-agents.md
- Claude Code 2.1.222 binary, bundled `dataviz` and `design-sync` skills (`~/.local/share/claude/versions/2.1.222`)
- `~/.claude/skills/meta-skill/` (SKILL.md, `reference/frontmatter-spec.md`, `reference/authoring-guide.md`, `reference/checklist.md`, `scripts/skill-lint.py`)
- `~/.claude/skills/writing-great-skills/` (SKILL.md, GLOSSARY.md)
- `/home/wil/orca/forge/.claude/skills/{forge-design,forge-tauri,playpen,playwright-cli}/`
- https://www.atlassian.com/blog/how-we-build/atlassians-design-md-is-here-what-we-learned-testing-portable-design-context-in-practice
- https://github.com/google-labs-code/design.md/blob/main/docs/spec.md

**Primary, fetched by sub-agent (not independently re-verified)**
- https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices
- https://ui.shadcn.com/docs/mcp · https://mui.com/material-ui/getting-started/mcp/ · https://github.com/Shopify/agent-skills
- https://www.atlassian.com/blog/ai-at-work/atlassian-design-system-building-the-context-engine-for-the-ai-era
- https://1password.com/blog/agent-driven-design-system
- https://github.com/triptease/claude-skill-design-system
- https://www.designsystems.one/ai-ready/systems
- https://arxiv.org/abs/2411.03292 (Interaction2Code) · https://mintviz.usv.ro/publications/2025.W4A.3.pdf (W4A 2025) · https://arxiv.org/html/2503.15885v1
- https://dl.acm.org/doi/10.1145/3772363.3799364 · https://arxiv.org/abs/2403.03163 (Design2Code) · https://arxiv.org/abs/2506.06251 (DesignBench)

**Tier C — weak, flagged inline**
- https://conesible.de/wab/whitepaper_webaccessbench.pdf (self-published, non-peer-reviewed)
- https://master.dev/blog/ai-generated-ui-is-inaccessible-by-default/ · https://blog.murphytrueman.com/the-parts-of-your-system-you-never-wrote-down/
- https://arxiv.org/pdf/2605.28840 (consistency; different domain, analogical only)
