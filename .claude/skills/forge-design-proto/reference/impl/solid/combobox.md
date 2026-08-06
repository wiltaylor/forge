# combobox — SolidJS

<!-- RECAST against the #66 template. This is the only one of the three combobox pages
     with an empty Contract-defects section. -->

control page: [combobox](../../controls/combobox.md)

## Shape

```
div.ffield
  span.ffield-label            when a label is given
  div.fcombo                   the dismiss root
    span.ffield-input          .is-error when errored
      <search glyph>
      input[role=combobox]
      <chevron glyph>
    div.fselect-pop[role=listbox]        when open
      div.fselect-opt[role=option]       .is-active .is-selected .is-disabled
        <label>
        span.fselect-check                on the selected one
      div.fcmd-empty                      instead, when nothing matches
  span.ffield-help             .is-error when errored
```

## What the browser gives you

Focus, tab order, scroll-into-view for the active option, and the whole ARIA vocabulary.
You write none of them. This is why this page is a third the length of the egui one for
the same control.

## Signals

Four: `open`, `query` (`string | null`, and `null` is the Contract's unset), `activeIdx`,
and the caller's `value` prop.

Do not destructure props. Read `props.options` inside the derivation so it stays reactive.

Derive `filtered` rather than storing it. Derive `selected` from `props.value` against
`props.options`; do not mirror it into a signal.

## Dismiss

The dismiss behaviour is a shared primitive, not per-control code. It takes the open
accessor, a close callback, and the root element. The close callback clears `query` as
well as closing — the Contract requires it, and missing it leaves a stale search string in
the field.

## Pointer and focus

`onPointerDown` on an option calls `preventDefault`, or the input blurs before the click
lands and the popup closes underneath the pointer.

`onFocus` opens the popup and selects the input's text, so the first keystroke replaces
the shown label.

## Keys

Every key handled in `onKeyDown` calls `preventDefault` — arrows otherwise move the text
caret, and Enter otherwise submits the surrounding form.

## Popup placement

The popup is a plain child of `.fcombo`, positioned by CSS. It does not go through the
overlay mount, because it is bound to the field, not to the screen.
