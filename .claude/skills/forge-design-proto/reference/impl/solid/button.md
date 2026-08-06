# button — SolidJS

<!-- RECAST against the #66 template. Short because the control is easy and the browser
     supplies most of it. Depth self-adjusts; no rule decides which controls get length. -->

control page: [button](../../controls/button.md)

## Shape

```
button.fbtn                  .fbtn-primary .fbtn-ghost .fbtn-danger .fbtn-sm
  <icon>                     optional, before the label
  <label>
  <icon>                     optional, after the label
```

## What the browser gives you

A real `<button>` gives you type, focus, the focus ring, Enter and Space, and the role.
You write none of them, which is the whole reason this page is four paragraphs and the
egui one is not.

## Then

Set `type="button"` unless it submits. The default is `submit`, which fires the
surrounding form.

`disabled` is the attribute. `loading` sets `disabled` as well and swaps the leading icon
slot for a spinner; keep the label mounted so the width does not change.

Icons come from `lucide-solid` at 1.5px stroke in `currentColor`.

Do not destructure props. Use `splitProps` to separate the Forge props from the ones that
pass through to the element.
