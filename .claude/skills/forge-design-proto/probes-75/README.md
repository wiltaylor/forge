# Firing probes — wayfinder #75

Primary source for [Does the settled description actually fire?](https://github.com/wiltaylor/forge/issues/75).
Findings are in `../PROBE-75.md`; this directory is the evidence they rest on.

Twenty probes, three runs each, on Opus 5. Each run was a fresh `claude` in a copy of one
substrate, with the sanitised skill in a shadow `CLAUDE_CONFIG_DIR` beside the 51 personal
skills — listed to the model, invisible to `ls`.

## Files

| Path | What it is |
|---|---|
| `probes.json` | The 20 asks with their predictions. **Fixed before any result was seen.** |
| `description.txt` | The exact 847-char description under test, byte for byte from #71 |
| `build_substrates.py` | Builds the seven substrates; asserts `plain-*` hold no `forge` |
| `build_reprobe.py` | The two substrates for the `P4`/`P9` re-probes |
| `run_probes.py` | The harness. Stops each run at the routing decision |
| `substrates/` | The nine probe projects, as handed to the agents |
| `results/raw-round1-main.json` | 51 runs — the 17 probes of the ticket's own list |
| `results/raw-round2-reprobe.json` | 9 runs — `P4b`, `P9b`, `P9c` |
| `results/raw-round3-followthrough.json` | 6 runs — `N4` and `X1` allowed to finish |
| `results/round*.log` | Live output of each round |

Each record carries the assistant text that preceded the decision, so *why* a probe fired
is recoverable, not just *that* it did.

## Substrates

| Directory | Family | Used by |
|---|---|---|
| `forge-solid` | Forge app | `P1`, `P7`, `P8`, `P9`, `P10`, `N4`, `N5`, `N6`, `X1` |
| `forge-solid-alerts` | Forge app | `P4b` — adds a visibly off-style alerts panel |
| `forge-solid-configs` | Forge app | `P9b`, `P9c` — adds two config versions |
| `forge-ratatui` | Forge app | `P2` |
| `forge-egui` | Forge app | `P3` |
| `forge-tauri` | Forge app | `P6` |
| `plain-axum` | no Forge trace | `P5`, `N1` |
| `plain-fastapi` | no Forge trace | `N2` |
| `plain-react` | no Forge trace | `N3` |

`P5` and `N1` share `plain-axum` on purpose: the same repo, two asks, and only the word
`Forge` between them. That pair is the discriminator test.

## Reproducing

```sh
python3 build_substrates.py     # writes /tmp/claude-1000/wf/substrates
python3 build_reprobe.py
python3 run_probes.py           # all probes; PROBE_FOLLOW=1 to run past the Skill call
python3 run_probes.py P2-ratatui-control   # or name probes
```

The harness needs the sanitised skill at `/tmp/claude-1000/wf/cfg/skills/forge-design` and
the personal skills symlinked beside it. **Do not run it from a path containing the string
`forge`** — an agent sees its cwd, and that contaminates every negative.

The run directories themselves are not kept; 66 sessions of scratch output is not evidence
anyone will read. `results/*.json` holds what each run decided and why.
