# button — SolidJS

<!-- PROTOTYPE (wayfinder #64). Short because the control is easy. Depth self-adjusts;
     no rule decides which controls deserve length. -->

status: complete · control page: [button](../../controls/button.md) ·
index: [solid](index.md)

```
button.fbtn                  .fbtn-primary .fbtn-ghost .fbtn-danger .fbtn-sm
  <icon>                     optional, before the label
  <label>
  <icon>                     optional, after the label
```

A real `<button>` element, so type, focus, Enter and Space are free. Set `type="button"`
unless it submits — the default is `submit`, which fires the surrounding form.

`disabled` is the attribute. `loading` sets `disabled` as well and swaps the leading icon
slot for a `spinner`; keep the label mounted so the width does not change.

Icons come from `lucide-solid` at 1.5px stroke in `currentColor`.

Do not destructure props. Use `splitProps` to separate the Forge props from the ones that
pass through to the element.
