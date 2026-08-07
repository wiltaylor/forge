# Does the settled description actually fire?

Findings from wayfinder [#75](https://github.com/wiltaylor/forge/issues/75). Twenty probes,
66 fresh Opus 5 sessions, each in a scratch project outside the repo.

[#71](https://github.com/wiltaylor/forge/issues/71) measured the description to the
character. Measurement proved it fits. This proves it fires.

## Verdict

**All three claims survive. The text does not change, and the 689 spare characters stay
spare.** Every positive fires 3/3. Every negative outside a Forge repo stays silent 0/3.
No probe found a phrase that would have bought recall, so there is nothing to buy.

One result reverses the ticket, not the design — see [The exclusion](#the-exclusion-the-ticket-set-the-wrong-bar).

## Method

Fixed before any result was seen; `probes-75/probes.json` holds the asks with their
predictions.

- **Three runs per probe**, so 3/3, 2/3 and 0/3 are distinguishable. Opus 5 throughout.
- **The skill was invisible to the working directory.** It sat in a shadow
  `CLAUDE_CONFIG_DIR` beside the 51 personal skills, which are the decoy field — 95
  commands listed, `forge-design` among them. This closed a leak the first smoke run
  exposed: with the skill on disk in `.claude/`, an agent ran `find .claude`, found the
  tree, and invoked it. That is routing by filesystem, not by description.
- **No path an agent could see contained the string `forge`** for the `plain-*`
  substrates. `build_substrates.py` asserts this and fails the build otherwise — it caught
  the scratchpad path itself, which contains `forge`, so the whole run tree was moved.
- **Substrate split, per the shape the effort settled**: positives run in a repo that is
  already a Forge app — hand-written components carrying Forge tokens and classes, as
  guidance-only implies. Negatives run in a plain axum, FastAPI or React project with no
  Forge trace. `X1` is the cross-check: a generic backend ask *inside* a Forge repo.
- **A run stopped at the routing decision** — the `Skill` call, or the first `Write`,
  which means the agent chose to build without consulting anything. Follow-through runs
  (round 3) were allowed to finish.
- The skill copy was sanitised: every `wayfinder` and `PROTOTYPE` marker stripped from 297
  files, so no probe could tell it was being tested. Isolation is honour-system, as in
  [#67](https://github.com/wiltaylor/forge/issues/67).

## Results

| Probe | Substrate | Expect | Fired | |
|---|---|---|---|---|
| `P1` named control, SolidJS | forge-solid | fire | **3/3** | pass |
| `P2` TUI ask, never names ratatui | forge-ratatui | fire | **3/3** | pass |
| `P3` desktop ask, never names egui | forge-egui | fire | **3/3** | pass |
| `P4` vague ask | forge-solid | fire | 0/3 | **void — no referent** |
| `P4b` vague ask, re-probe | forge-solid-alerts | fire | **3/3** | pass |
| `P5` Forge doc store in axum | plain-axum | fire | **3/3** | pass |
| `P6` Tauri v2 packaging and IPC | forge-tauri | fire | **3/3** | pass |
| `P7` chat UI | forge-solid | fire | **3/3** | pass |
| `P8` add a chart | forge-solid | fire | **3/3** | pass |
| `P9` show a diff | forge-solid | fire | 0/3 | **void — no referent** |
| `P9b` show a diff, referent present | forge-solid-configs | fire | 0/3 | correct non-fire |
| `P9c` show a diff **in the UI** | forge-solid-configs | fire | **3/3** | pass |
| `P10` build a node editor | forge-solid | fire | **3/3** | pass |
| `N1` generic axum SSE | plain-axum | silent | **0/3** | pass |
| `N2` generic FastAPI | plain-fastapi | silent | **0/3** | pass |
| `N3` React component | plain-react | silent | **0/3** | pass |
| `N4` terminal emulator | forge-solid | silent | 3/3 | see below |
| `N5` VNC viewer | forge-solid | silent | 1/3 | see below |
| `N6` RDP viewer | forge-solid | silent | 2/3 | see below |
| `X1` generic SSE inside a Forge repo | forge-solid | silent | 3/3 | see below |

No probe ever invoked a different skill. `dataviz` was in the decoy field and never took
`P8`.

## Claim 1 — the Rust platforms fire

**Confirmed.** `P2` asked for *"a settings screen for the TUI"* and `P3` for *"a data table
to the desktop app"*. Neither names its library. Both fire 3/3, and the reasoning is
explicit — *"This is a Forge-styled ratatui app, so I'll load the design system first."*

The bet #71 made — that the model's prior binds `ratatui` to TUI work and `egui` to desktop
work — holds. 63 characters of platform vocabulary do the job they were bought for.

## Claim 2 — `Forge backend contract` discriminates, and more sharply than #71 supposed

**Confirmed, with a correction to how it works.**

`N1` and `P5` run in the **same substrate**, a plain axum service with no Forge anywhere.
Only the sentence differs:

| Ask | Fired |
|---|---|
| *"Add an SSE endpoint to my axum server."* | **0/3** |
| *"Wire up the Forge doc store in axum."* | **3/3** |

That is the cleanest result in the run. The word `Forge` is doing the discriminating,
exactly as designed, and the riskiest line in the description is sound.

**`X1` refines it: the word does not have to be in the ask.** The same generic SSE ask,
moved into a Forge repo, fires 3/3 — *"This repo uses the Forge design system, and the task
is an axum SSE endpoint — that's squarely in the `forge-design` skill's scope."* So the
discriminator keys on the **situation**, not the sentence. A Forge app's server should
follow the Forge backend contract, so this is the right answer, not a false fire.

**Caveat that limits both:** `reference/api/index.md` says *"Not written."* `P5` and `X1`
route there correctly and land on a stub. These probes prove backend routing **fires**;
they cannot prove it is **useful**. Filling it belongs to
[Extraction plan and the deletion gate](https://github.com/wiltaylor/forge/issues/68).

## Claim 3 — the dropped phrases cost no recall

**Confirmed.** Twelve of fifteen quoted triggers were cut in #71, on the claim that a prose
noun fires as well as a quoted phrase. It does:

| Dropped phrase | Probe ask | Fired |
|---|---|---|
| `"chat UI"` | *Build a chat UI with an assistant transcript.* | 3/3 |
| `"add a chart"` | *Add a chart showing request latency over the last hour.* | 3/3 |
| `"build a node editor"` | *Build a node editor for the pipeline graph.* | 3/3 |
| `"show a diff"` | *Show a diff between the two config versions in the UI.* | 3/3 |

The named-hits clause is earning its characters. Nothing needs buying back.

### `P9b` is a correct non-fire, and worth keeping

*"Show a diff between the two config versions"* — with the two versions present and no UI
intent — does **not** fire, 3/3. All three agents printed a unified diff at the terminal,
which is what was asked. Adding *"in the UI"* fires 3/3.

So the `code and diff editor` noun does not over-trigger on a plain diff request. That is
the discrimination you want, and it is the one place in the run where a positive probe
failing is the good outcome.

## The exclusion — the ticket set the wrong bar

The ticket listed terminal, VNC and RDP as probes that **must not fire**. They fire 6/9
inside a Forge repo. That is not a defect, and the ticket's bar contradicts the design it
was testing: #71 settled that *"the refusal stays in `SKILL.md`, read after firing."*

**The description cannot exclude the vertical, and no wording would.** Every fire gave the
same reason, and none of them mentioned a terminal: *"This is a Forge design system UI, and
I'm adding a pane to it."* The trigger that fires is `building or styling a Forge UI` — the
clause the skill exists for. A negative cannot cancel it without cancelling the skill.

**Measured follow-through, letting all three `N4` runs finish: the refusal works, 3/3, with
zero files written.** Each one read `SKILL.md`, checked the control list, checked
`gaps.md`, found no row, and stopped:

> A terminal pane is therefore out of the design system, not merely unbuilt in SolidJS. If
> I built it from the Forge pages, I would have to invent class and token names.

The cost of the detour is **$0.33–$0.47 and 13–18 tool calls**. The failure it prevents is
a pane that looks like a Forge control and carries no token, keyboard or focus contract —
which one agent named unprompted.

`N5` at 1/3 and `N6` at 2/3 are the same behaviour, less consistently reached. The
non-firing `N6` run went straight to writing a `guacd` bridge without consulting anything —
a miss, but not one the description could have prevented.

**Recorded as correct behaviour. Silence stands, and it works because the body carries the
refusal — not because the description excludes anything.**

## Two probes of the ticket's own list were void

`P4` and `P9` failed 0/3 for a reason that says nothing about the description: the thin
substrate held no alerts panel and no second config version. All six agents searched,
reported the referent missing, and stopped — correctly. Re-run with the referent present,
`P4b` fires 3/3.

**A firing probe needs the thing its ask points at.** An ask that references an existing
artifact measures nothing without one. Worth carrying into any future routing probe.

`P9c` amends its ask to add *"in the UI"*, and says so — the original is kept and reported
as `P9b` so the amendment cannot hide a result.

## What this does not reach

- **One model.** Opus 5 only. A weaker router may behave differently, and the first smoke
  run on Sonnet 5 showed the same fire on `P2`, but that is one data point, not a result.
- **Routing after firing** is #64's, closed.
- **Whether the destination is worth reaching.** The catalogue is stubs on this branch.
  `P5` and `X1` prove the door opens, not that the room is furnished.
- **Isolation is honour-system.** A probe that peeked cannot be caught.

Evidence in `probes-75/`. 66 sessions, $6.03 of recorded cost — early-stopped runs report
none, so the true figure is higher.
