# combobox — SolidJS

<!-- PROTOTYPE (wayfinder #64). -->

status: complete · control page: [combobox](../../controls/combobox.md) ·
index: [solid](index.md)

Read the control page first. It owns the state model and the interaction contract. This
page owns only what SolidJS adds.

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

Focus, tab order, scroll-into-view for the active option, and the ARIA vocabulary. You
write none of them. This is why this page is short and the egui one is not.

## Solid specifics

Four signals: `open`, `query` (`string | null`, and `null` is "unset" — see the control
page), `activeIdx`, and the caller's `value` prop.

Do not destructure props. Read `props.options` inside the derivation so it stays reactive.

Derive `filtered` rather than storing it. Derive `selected` from `props.value` against
`props.options`; do not mirror it into a signal.

The dismiss behaviour is a shared primitive, not per-control code. It takes the open
accessor, a close callback, and the root element. The close callback clears `query` as
well as closing — see the control page on why.

`onPointerDown` on an option calls `preventDefault`, or the input blurs before the click
lands and the popup closes underneath the pointer.

`onFocus` opens the popup and selects the input's text, so the first keystroke replaces
the shown label.

Every key handled in `onKeyDown` calls `preventDefault` — arrows otherwise move the text
caret, and Enter otherwise submits the surrounding form.

The popup is a plain child of `.fcombo`, positioned by CSS. It does not go through the
overlay mount, because it is bound to the field, not to the screen.
