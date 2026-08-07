# Tokens

<!-- Filled for wayfinder #67 by mechanical extraction from packages/tokens/css/tokens.css,
     crates/forge-tui/src/theme/mod.rs and crates/forge-egui/src/theme/{mod,tokens}.rs.
     Transcription only — the source wins, warts included, and nothing here is renamed.
     Shape follows #73: one page, three platforms, `role | SolidJS | ratatui | egui`. -->

`laws.md` names the roles. This page gives the names you actually type. Read it when you
need a value; you do not need it in your head.

Every value is a token reference. No literals, ever, and never a name that is not below.

## Colour

SolidJS spells a CSS custom property — `var(--bg-1)`. Both Rust kits spell a field on
`Theme`, which is passed down from the app root.

| Role (laws.md) | SolidJS | ratatui | egui |
|---|---|---|---|
| `surface` — page | `--bg-0` | `theme.bg[0]` | `theme.bg[0]` |
| `surface-raised` — card | `--bg-1` | `theme.bg[1]` | `theme.bg[1]` |
| hover / nested card | `--bg-2` | `theme.bg[2]` | `theme.bg[2]` |
| pressed / active row | `--bg-3` | `theme.bg[3]` | `theme.bg[3]` |
| popover, dropdown | `--bg-4` | `theme.bg[4]` | `theme.bg[4]` |
| `text` — primary | `--fg-0` | `theme.fg[0]` | `theme.fg[0]` |
| secondary text | `--fg-1` | `theme.fg[1]` | `theme.fg[1]` |
| `text-dim` — tertiary, captions | `--fg-2` | `theme.fg[2]` | `theme.fg[2]` |
| disabled, placeholder | `--fg-3` | `theme.fg[3]` | `theme.fg[3]` |
| `border` — 1px divisions | `--border` | `theme.border.default` | `theme.border.default` |
| hairline division | `--border-subtle` | `theme.border.subtle` | `theme.border.subtle` |
| emphasised division | `--border-strong` | `theme.border.strong` | `theme.border.strong` |
| `accent` | `--accent` | `theme.accent.base` | `theme.accent.base` |
| accent hover | `--accent-hover` | `theme.accent.hover` | `theme.accent.hover` |
| accent pressed | `--accent-press` | `theme.accent.press` | `theme.accent.press` |
| accent tint (14%) | `--accent-bg` | `theme.accent.bg` | `theme.accent.bg` |
| accent text | `--accent-fg` | `theme.accent.fg` | `theme.accent.fg` |
| text on solid accent | `--accent-contrast` | `theme.accent.contrast` | `theme.accent.contrast` |

`surface-sunken` is named by `laws.md` but has no token of its own. Wells, code blocks and
log bodies take `--bg-0` inside a raised container.

### Status

Four statuses, each a triple. Note the spellings: `laws.md` says `ok` and `warn`; the
tokens say `success` and `warning`. The tokens win — they are what you type.

| Status | SolidJS | Rust (both kits) |
|---|---|---|
| `ok` | `--success` `--success-bg` `--success-fg` | `theme.success.{base,bg,fg}` |
| `warn` | `--warning` `--warning-bg` `--warning-fg` | `theme.warning.{base,bg,fg}` |
| `danger` | `--danger` `--danger-bg` `--danger-fg` | `theme.danger.{base,bg,fg}` |
| `info` | `--info` `--info-bg` `--info-fg` | `theme.info.{base,bg,fg}` |

`base` is the solid. `bg` is a 14% tint for a filled badge or alert. `fg` is the text that
goes on that tint. Rust also reaches these through `theme.severity(Severity::Danger)`.

Both Rust kits carry the same colour fields, in the same order, deliberately: the struct
comment says the layout mirrors the web `Theme` interface.

## Geometry

The ratatui column is empty on purpose, and the emptiness is the statement — a terminal has
cells, not pixels. Sizing there is rows and columns.

| Role | SolidJS | ratatui | egui |
|---|---|---|---|
| radius small | `--r-sm` (4px) | — | `theme.radius.sm` (4.0) |
| radius medium | `--r-md` (6px) | — | `theme.radius.md` (6.0) |
| radius large | `--r-lg` (8px) | — | `theme.radius.lg` (8.0) |
| radius pill | `--r-pill` (999px) | — | `height / 2.0` at the call site |
| spacing step | `--sp-1` … `--sp-16` | — | `theme.space.x(n)` = n × 4.0 |
| control height sm | `--h-sm` (28px) | — | `theme.control.sm` (28.0) |
| control height md | `--h-md` (32px) | — | `theme.control.md` (32.0) |
| control height lg | `--h-lg` (36px) | — | `theme.control.lg` (36.0) |
| control height xl | `--h-xl` (40px) | — | `theme.control.xl` (40.0) |
| sidebar width | `--sidebar-w` (240px) | — | `SIDEBAR_WIDTH` (240.0) |
| sidebar rail | — | — | `SIDEBAR_RAIL` (56.0) |
| top bar height | `--topbar-h` (48px) | — | `TOPBAR_HEIGHT` (48.0) |
| status bar height | — | — | `STATUSBAR_HEIGHT` (28.0) |

The SolidJS spacing scale is `--sp-1` 4px, `-2` 8, `-3` 12, `-4` 16, `-5` 20, `-6` 24,
`-8` 32, `-10` 40, `-12` 48, `-16` 64. There is no `--sp-7`, `-9`, `-11` or `-13..15`.

## Type

| Role | SolidJS | ratatui | egui |
|---|---|---|---|
| sans family | `--font-sans` | — | `theme.font(ctx, weight, size)` |
| mono family | `--font-mono` | — | `theme.mono(size)` |
| 11px | `--fs-xs` | — | `theme.type_scale.xs` |
| 12px | `--fs-sm` | — | `theme.type_scale.sm` |
| 14px body | `--fs-base` | — | `theme.type_scale.base` |
| 16px | `--fs-md` | — | `theme.type_scale.md` |
| 18px | `--fs-lg` | — | `theme.type_scale.lg` |
| 22px | `--fs-xl` | — | `theme.type_scale.h3` |
| 28px | `--fs-2xl` | — | `theme.type_scale.h2` |
| 34px | `--fs-3xl` | — | `theme.type_scale.h1` |

Note the mismatch, and do not tidy it: the CSS heading sizes are `--fs-xl/2xl/3xl` and the
egui ones are `h3/h2/h1` for the same 22/28/34.

Line height `--lh-tight` 1.2, `--lh-snug` 1.4, `--lh-normal` 1.5, `--lh-relaxed` 1.65.
Weight `--fw-regular` 400, `--fw-medium` 500, `--fw-semibold` 600, `--fw-bold` 700; egui
takes `FontWeight::{Regular, Medium, SemiBold}` and ships no bold. Tracking is
`--tracking-tight`, `-normal`, `-wide`, `-eyebrow`.

## Motion

| Role | SolidJS | ratatui | egui |
|---|---|---|---|
| fast | `--dur-1` (80ms) | — | `theme.motion.fast` (0.08s) |
| base | `--dur-2` (160ms) | — | `theme.motion.base` (0.16s) |
| slow | `--dur-3` (240ms) | — | `theme.motion.slow` (0.24s) |
| easing | `--ease-out` | — | applied at the call site |

## Shadow

`--shadow-sm` and `--shadow-md` both resolve to `none`, in both themes. Forge separates
layers with a surface step and a border, not with a shadow.

## Themes

Dark is the default and lives on `:root`. Light arrives two ways —
`@media (prefers-color-scheme: light)` on `:root:not([data-theme="dark"])`, and
`[data-theme="light"]` on any element, so two panes can show both themes at once. Rust
takes `Theme::dark()` or `Theme::light()`.

`Theme::with_accent(base)` derives hover, press, tint and text from one brand colour on
both Rust kits. ratatui adds `Theme::quantized(mode)` for 256- and 16-colour terminals.

## Breakpoints

CSS custom properties cannot be used inside a `@media` condition, so these are constants
you type as literals — the one documented exception to the no-literals rule:

- compact `@media (max-width: 1024px)` — the sidebar becomes a drawer
- mobile `@media (max-width: 768px)` — single-column stacking
- `@media (pointer: coarse)` raises the `--h-*` control heights
