# Data Explorer Template

Use this template when the playpen is about data queries, APIs, pipelines, or
structured configuration: SQL builders, API designers, regex builders, pipeline
visuals, cron schedules.

## Layout

Standard `pp-shell` grid: controls (left) → formatted output (right, in a `Card` with
`padded={false}` around a `<pre>`) → prompt output (bottom). Group controls per stage of
the query/pipeline (Source, Fields, Filters, Grouping, Ordering, Limits).

## Control types

| Decision | Control | Example |
|---|---|---|
| Select from available items | Clickable `Badge`s or chip buttons | table names, columns, HTTP methods |
| Add filter/condition rows | `Button icon={Plus}` → row of selects + input | WHERE column op value |
| Join type or aggregation | `<select>` per row | INNER/LEFT/RIGHT, COUNT/SUM/AVG |
| Limit/offset | Range slider | result count 1–500 |
| Ordering | `<select>` + ASC/DESC toggle button | order by column |
| On/off features | Checkbox | show descriptions, include header |

Selected chips: `fbadge fbadge-accent`; unselected: `fbadge fbadge-neutral`.

## Output rendering

Build the generated artifact (SQL, spec, regex) with a `createMemo`, render it
syntax-highlighted with token colours:

```jsx
const sql = createMemo(() => buildSql(state));

const highlighted = createMemo(() =>
  sql()
    .replace(/\b(SELECT|FROM|WHERE|JOIN|ON|GROUP BY|ORDER BY|LIMIT)\b/g,
      '<span class="kw">$1</span>')
    .replace(/'[^']*'/g, '<span class="str">$&</span>'));

<pre class="pp-prompt-text" innerHTML={highlighted()} />
```

```css
/* playpen.css additions — token colours only */
.kw  { color: var(--accent-fg); font-weight: var(--fw-semibold); }
.str { color: var(--success-fg); }
.tbl { color: var(--info-fg); }
```

For pipeline-style playpens, render a flow of `.fcard` steps joined by
`<ArrowRight size={14} />` (lucide-solid) in a flex row.

## Data & endpoints

| Document / action | Purpose |
|---|---|
| `data: state` | Query/pipeline control state |
| `data: schema` | Table/column definitions the controls populate from — seed it with `playpen data set schema --file …` |
| `action: execute` | Optional — actually run the query/pipeline step server-side |
| `action: export` | Optional — write the artifact (SQL file, OpenAPI spec) into the repo (validate paths, see `reference/data-api.md`) |

## Prompt output

Frame it as a specification of what to build, not the raw query:

> "Write a SQL query that joins orders to users on user_id, filters for orders after
> 2024-01-01 with total > $50, groups by user, and returns the top 10 users by order
> count."

Include schema context so the prompt is self-contained.

## Example topics

- SQL query builder (tables, joins, filters, group by, order by, limit)
- API endpoint designer (routes, methods, request/response field builder)
- Data transformation pipeline (source → filter → map → aggregate → output)
- Regex builder (sample strings, match groups, live highlight)
- Cron schedule builder (visual timeline, interval, day toggles)
- GraphQL query builder (type selection, field picker, nested resolvers)
