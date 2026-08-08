# Testing SolidJS components

Most npm packages in this workspace test pure functions in a Node environment:
markdown parsing, layout arithmetic, board moves, wire decoding. Those tests
stay as they are.

This document covers the other layer. It mounts a component, dispatches a real
event, and asserts on the resulting document. `@forge/ui` is the reference
setup. Copy it into any package that needs it.

## What the setup is

Three parts:

1. **A DOM.** `happy-dom` gives the test a `document`, so a component has
   somewhere to render.
2. **A JSX transform.** `vite-plugin-solid` compiles the `.tsx` test file and
   the component source with Solid's transform. Without it, JSX in a test does
   not compile.
3. **Solid's testing library.** `@solidjs/testing-library` mounts a component
   into a container, unmounts it again, and re-exports the queries and the
   event helpers of `@testing-library/dom`.

## Adopting it in a package

Add the dev dependencies. Include `vitest` if the package does not have it:

```sh
pnpm --filter @forge/<package> add -D vitest happy-dom vite vite-plugin-solid @solidjs/testing-library
```

Add the test script to `package.json`, so the aggregate `turbo test` run picks
the package up:

```jsonc
"scripts": {
  "test": "vitest run"
}
```

Write `vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  /* The @forge/* deps ship preserved-JSX source under their `solid` export
     condition; one Solid runtime, or reactivity breaks across the boundary. */
  resolve: { dedupe: ['solid-js'] },
  test: {
    environment: 'happy-dom',
    setupFiles: ['tests/setup.ts'],
  },
});
```

Write `tests/setup.ts`. If the package already has a setup file, add these
lines to it. Do not replace the file:

```ts
import { afterEach } from 'vitest';
import { cleanup } from '@solidjs/testing-library';

afterEach(cleanup);
```

The `cleanup` call is not optional. Solid's testing library registers it for
you only when `afterEach` is a global, and no package here enables Vitest
globals. Without the setup file, one test's document leaks into the next.

The `dedupe` entry is not optional either. A `@forge/*` dependency resolves to
preserved-JSX source through its `solid` export condition. A second copy of
`solid-js` breaks the boundary between them. Signals made in one copy do not
drive components built by the other, and the component looks inert.

Name component test files `*.test.tsx`. A `.ts` file cannot hold JSX.

Each package type-checks `src` only, so `tsc --noEmit` does not look at the
test files. Vitest strips their types without checking them, so nothing
reports a type error in a test. Your editor is the only check on it.

## Writing a test

```tsx
import { describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import { fireEvent, render, screen } from '@solidjs/testing-library';
import { Toggle } from '../src/forms';

describe('Toggle', () => {
  it('shows the state its owner settles on', () => {
    const [on, setOn] = createSignal(false);
    render(() => (
      <Toggle checked={on()} onChange={setOn}>
        Dark mode
      </Toggle>
    ));

    const el = screen.getByRole<HTMLInputElement>('switch', { name: 'Dark mode' });
    fireEvent.click(el);

    expect(el.checked).toBe(true);
  });
});
```

Note the shape:

- **`render` takes a function**, not an element. Solid components run once, and
  the function is the reactive scope the library owns.
- **Query by role and accessible name**, the way a user or a screen reader
  finds the control. A query by CSS class asserts on the implementation. It
  will break for the wrong reason when the markup changes.
- **Dispatch a real event.** Call `fireEvent.click`, not the handler prop. The
  handler is the implementation; the click is what the user does.
- **Assert on the document** — what is there, what has focus, what an assistive
  technology is told. Assert on a callback only for what leaves the component
  and never reaches the document. Never assert on an internal signal, or on
  which handler ran.

Solid updates the DOM synchronously, so an assertion directly after
`fireEvent` sees the new document. Use `findBy*` queries only for work that is
genuinely asynchronous.

## What does not work

- **Layout.** happy-dom computes no geometry: every element measures zero. A
  component reading `getBoundingClientRect` needs the test to supply that
  input.
- **Observers.** `ResizeObserver` and `IntersectionObserver` exist, but they
  never call back: there is no layout to report a change in. A component that
  waits for one before it does its work needs the test to drive that work
  another way.
- **Painting.** happy-dom applies stylesheets, so `getComputedStyle` gives a
  correct answer. It paints nothing, and no geometry follows from a style. The
  gallery stays the visual check.

## The one other package with a DOM environment

`@forge/chat` set up `happy-dom` and `vite-plugin-solid` before this document
existed. It mounts with `render` from `solid-js/web` directly and disposes by
hand, and its `tests/setup.ts` holds an unrelated `ResizeObserver` stub. It
works, so it is left alone. Follow this document for new tests, in that
package as well as in the others.
