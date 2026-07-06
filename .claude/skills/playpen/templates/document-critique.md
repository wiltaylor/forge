# Document Critique Template

Use this template when the playpen helps review and critique documents: SKILL.md
files, READMEs, specs, proposals — any text needing structured feedback with an
approve/reject/comment workflow.

## Layout

Adapt `pp-shell` to a document/suggestions split: document with line numbers (left,
wide), suggestions panel (right), prompt output (bottom, full width). Override the grid
in `playpen.css`:

```css
.pp-shell.pp-critique { grid-template-columns: 1fr 360px; grid-template-areas:
  "doc suggestions" "prompt prompt"; }
```

## Key components

**Document panel** — `Card padded={false}`, lines rendered with `<For>`, mono font,
line numbers in `--fg-3`. Lines with suggestions get a status left border via
`classList`; clicking a suggestion card scrolls its line into view
(`el.scrollIntoView({block: 'center'})`).

**Suggestions panel** — filter row of `Button size="sm"` (All / Pending / Approved /
Rejected) with counts as `Badge`s; each suggestion is a `Card` with the line ref
(mono), the suggestion text, Approve / Reject `Button`s and an optional comment
`textarea` in `.ffield-input`.

**Prompt output** — built only from approved suggestions + user comments, grouped:
Approved improvements / Additional feedback / Rejected (for context).

## Status styling — tone triples, not raw colours

```css
.doc-line.pending  { border-left: 3px solid var(--warning); background: var(--warning-bg); }
.doc-line.approved { border-left: 3px solid var(--success); background: var(--success-bg); }
.doc-line.rejected { border-left: 3px solid var(--danger);  background: var(--danger-bg); opacity: 0.6; }
```

Status badges: `<Badge tone="warning" dot>pending</Badge>` /
`tone="success"` / `tone="danger"`.

## State structure

```jsx
const [state, setState] = createStore({
  suggestions: [
    { id: 1, lineRef: 3, targetText: 'description: …', category: 'clarity',
      suggestion: 'The description is too long.', status: 'pending', userComment: '' },
  ],
  activeFilter: 'all',
  activeSuggestionId: null,
});
```

Update one suggestion with path syntax, then persist the user-authored output to its
own document so the agent can read it back cleanly:

```jsx
const setStatus = (id, status) => {
  setState('suggestions', (s) => s.id === id, 'status', status);
  saveDocDebounced('comments', unwrap(state).suggestions);
};
```

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: document` | The document text — seed with `playpen data set document --file …` |
| `data: comments` | Suggestion statuses + user comments — **the agent reads this back** with `playpen data get comments` |
| `data: state` | UI state (filter, selection) |
| `action: apply` | Optional — apply approved suggestions to the real file (path-validate; ask the user first) |

## Pre-populating

When building for a specific document: read it, generate suggestions with line
references yourself, then seed both documents before handing over the URL:

```
uv run .playpen/playpen.py data set document --file README.md.json
uv run .playpen/playpen.py data set comments --file suggestions.json
```

## Prompt output

> "Apply these approved improvements to README.md: (1) Line 3 — shorten the
> description to one sentence. (2) Line 41 — add an example invocation. Additional
> feedback: the install section assumes npm; mention uv too. (Rejected, for context:
> renaming the project.)"

## Example topics

- SKILL.md review (definition quality, completeness, clarity)
- README critique (documentation quality, missing sections)
- Spec review (requirements clarity, missing edge cases)
- Proposal feedback (structure, argumentation, missing context)
