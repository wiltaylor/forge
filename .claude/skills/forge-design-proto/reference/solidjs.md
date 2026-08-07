# SolidJS class grammar

<!-- Added for wayfinder #67, to the shape #73 settled: class ROOTS, not the ~364 leaf
     classes. Roots transcribed from the `.f*` selectors in packages/**/*.css. Forward
     lookup is zero-hop from an implementation page's Shape block, so this page earns its
     place in reverse — you have a class and need to know what owns it. -->

Read this once a session, before the first SolidJS control.

## The grammar

Every control class starts `f`, then the control's root, then a hyphenated part:

```
.f<root>              the control's own element
.f<root>-<part>       a named part of it
.is-<state>           a state, always as a second class, never merged into the root
```

`.is-active`, `.is-selected`, `.is-open`, `.is-disabled`, `.is-error` are the states you
will meet most. A state class never appears alone — it modifies an `f`-class on the same
element.

Nothing is minted. A class you cannot find in a control's Shape block does not exist; say
so in your summary rather than inventing one. **A new root is never abbreviated** —
`tree` becomes `.ftree`. The legacy irregulars below (`.facc`, `.fbtn`, `.fcal` sitting
beside `.favatar`, `.fbadge`) are recorded, and never extended.

## The ten unprefixed classes

Exactly ten classes carry no `f` prefix. They are the screen skeleton, and `laws.md` names
every one of them:

```
app-shell   app-main   page-head   page-actions   empty   eyebrow
settings-layout   settings-nav   settings-section   settings-row
```

Do not apply the `f` grammar to these. They are the one family exempt from it.

## The roots

| Root | Owner |
|---|---|
| `facc` | accordion |
| `falert` | alert |
| `favatar` | avatar |
| `fbadge` | badge |
| `fbtn` | button, icon-button |
| `fcal` | calendar |
| `fcard` | card |
| `fchart` | pie-chart, line-chart, bar-chart, gantt-chart, sparkline (shared frame) |
| `fchat` | the chat kit — chat-view, chat-message, chat-composer, chat-divider, chat-typing |
| `fcheck` | checkbox |
| `fcmd` | command, and the shared empty line `.fcmd-empty` |
| `fcode` | code-editor, diff-editor |
| `fcombo` | combobox |
| `fctx` | context-menu |
| `fdate` | date-picker |
| `fdot` | status-dot |
| `ffield` | the shared form-field wrapper — label, input frame, help text |
| `fflow` | flowchart |
| `ffx` | fx-layer |
| `fgantt` | gantt-chart internals |
| `fgraph` | node-graph |
| `fkbd` | kbd |
| `flistbox` | list-box |
| `flog` | log-line |
| `flogs` | logs |
| `fmd` | markdown |
| `fmenu` | dropdown-menu |
| `fmodal` | modal |
| `fpage` | pagination |
| `fpalette` | chart palette swatches |
| `fpop` | the shared popup surface — `.fselect-pop` and friends build on it |
| `fpopover` | popover |
| `fprogress` | progress |
| `fradio` | radio, radio-group |
| `fscrim` | the overlay backdrop |
| `fseg` | toggle-group |
| `fselect` | select, and the option rows combobox reuses |
| `fsep` | separator |
| `fsheet` | sheet |
| `fsidebar` | app-shell's nav — nav-section, nav-link |
| `fskel` | skeleton |
| `fslider` | slider |
| `fspinner` | spinner |
| `fsplit` | split-pane |
| `fstat` | stat |
| `ftab` / `ftabs` | tabs |
| `ftable` | table |
| `ftip` | tooltip |
| `ftoast` / `ftoaster` | toast, toaster |
| `ftoggle` | toggle |
| `ftopbar` | app-shell's top bar — crumbs live here |

Outside the settled catalogue, and listed only so you recognise them if you meet one:
`fbk` (block editor), `fblockgrid`, `fgrid`, `fkanban`, `fterm`, `fdesk`.

## Two things that surprise people

`combobox` does not own its popup rows. The field is `.fcombo`, but the options are
`.fselect-opt` inside `.fselect-pop` — shared with `select`. Reuse them; do not mint
`.fcombo-opt`.

`.ffield` wraps every form control that has a label and help text. The label is
`.ffield-label`, the input frame is `.ffield-input`, the help line is `.ffield-help`, and
the error state is `.is-error` on the frame and the help line both.
