/* Solid's testing library registers its own cleanup only when `afterEach` is a
   global; this suite does not enable Vitest globals, so unmount each render
   here. Without it, one test's document leaks into the next. */
import { afterEach } from 'vitest';
import { cleanup } from '@solidjs/testing-library';

afterEach(cleanup);

/* ChatView observes its list to stay pinned to the bottom; the test DOM has
   no layout, so a no-op observer is all that is needed. */
if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
}
