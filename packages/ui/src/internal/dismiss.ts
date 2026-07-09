import { createEffect, onCleanup } from 'solid-js';

/** Shared dismiss wiring for anchored popovers: click-outside + Escape. */
export function useDismiss(
  open: () => boolean,
  close: () => void,
  root: () => HTMLElement | undefined,
): void {
  createEffect(() => {
    if (!open()) return;
    const onDown = (e: PointerEvent) => {
      const el = root();
      if (el && !el.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('pointerdown', onDown);
    document.addEventListener('keydown', onKey);
    onCleanup(() => {
      document.removeEventListener('pointerdown', onDown);
      document.removeEventListener('keydown', onKey);
    });
  });
}
