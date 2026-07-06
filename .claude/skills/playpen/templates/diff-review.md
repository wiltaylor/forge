# Diff Review Template

Use this template when the playpen is about reviewing code diffs: git commits, pull
requests, code changes with interactive line-by-line commenting.

## Layout

Adapt `pp-shell`: commit header + file list (left, narrow), diff content (right, wide,
mono), prompt output (bottom). Diff lines render like the Forge log viewer — dense
mono rows in a `Card padded={false}`.

## Interactions

| Feature | Control | Behavior |
|---|---|---|
| Line commenting | Click any diff line | Opens a `textarea` row under the line |
| Comment indicator | `Badge tone="accent"` on commented lines | Shows which lines have feedback |
| Save/Cancel | `Button size="sm"` pair in the comment row | Persist or discard |
| Copy prompt | Copy button in prompt panel | All comments as review text |

## Diff data structure

```jsx
// data: diff — seeded by the agent (see below)
[{
  file: 'path/to/file.py',
  hunks: [{
    header: '@@ -41,13 +41,13 @@ def context',
    lines: [
      { type: 'context',  oldNum: 41, newNum: 41, content: 'unchanged' },
      { type: 'deletion', oldNum: 42, newNum: null, content: 'removed' },
      { type: 'addition', oldNum: null, newNum: 42, content: 'added' },
    ],
  }],
}]
```

## Line styling — tone triples

```css
.diff-line { font-family: var(--font-mono); font-size: var(--fs-sm); display: grid;
             grid-template-columns: 48px 48px 1fr; padding: 1px var(--sp-3); }
.diff-line.addition    { background: var(--success-bg); }
.diff-line.deletion    { background: var(--danger-bg); }
.diff-line.hunk-header { background: var(--info-bg); color: var(--info-fg); }
.diff-line.commented   { box-shadow: inset 2px 0 0 var(--accent); }
.diff-num { color: var(--fg-3); text-align: right; padding-right: var(--sp-2); }
```

Render with nested `<For>` (files → hunks → lines); `classList={{ commented:
!!comments[lineId(line)] }}`.

## Comment system

Comments live in their own store and document (the agent reads them back):

```jsx
const [comments, setComments] = createStore({}); // lineId → text

const saveComment = (lineId, text) => {
  text.trim() ? setComments(lineId, text.trim()) : setComments(lineId, undefined);
  saveDocDebounced('comments', unwrap(comments));
};
```

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: diff` | Parsed diff structure — seeded by the agent |
| `data: comments` | lineId → comment — **read back with `playpen data get comments`** |
| `action: load-diff` | Optional — server runs git to load a commit on demand |

Loading a real diff server-side (register in `ACTIONS`):

```python
import subprocess

def load_diff(payload: dict):
    commit = payload.get("commit", "HEAD")
    out = subprocess.run(
        ["git", "show", commit, "--format=%H%n%s%n%an%n%ad", "-p"],
        capture_output=True, text=True, cwd=SERVER_DIR.parent.parent,
    ).stdout
    return parse_diff(out)   # you write parse_diff → the structure above
```

Simplest path: the agent parses `git show` itself and seeds
`playpen data set diff --file parsed.json` — no server change needed.

## Prompt output

```
Code review comments:

file.py:45
  Code: subagent_id = tracker.register()
  Comment: Consider adding error handling for registration failures.

file.py:52
  Code: return result.data
  Comment: Should validate result is not None before accessing .data
```

## Example topics

- Git commit review (single commit diff with line comments)
- Pull request review (multiple commits, file-level and line-level comments)
- Code diff comparison (before/after refactoring)
- Code audit (security review with findings per line)
