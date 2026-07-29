/* ChatView observes its list to stay pinned to the bottom; the test DOM has
   no layout, so a no-op observer is all that is needed. */
if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
}
