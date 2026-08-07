import { createEffect, onCleanup } from 'solid-js'

/**
 * The shared dismiss primitive. It takes the open accessor, a close callback, and
 * the root element.
 *
 * A pointer press or a focus move outside the root dismisses. The close callback
 * does the rest of the work — for a combobox it clears the query too, because
 * dismissal by any route behaves as Escape.
 */
export function createDismiss(
  isOpen: () => boolean,
  close: () => void,
  root: () => HTMLElement | undefined,
): void {
  const outside = (target: EventTarget | null): boolean => {
    const el = root()
    return !!el && !(target instanceof Node && el.contains(target))
  }

  const onPointerDown = (event: PointerEvent) => {
    if (outside(event.target)) close()
  }

  const onFocusIn = (event: FocusEvent) => {
    if (outside(event.target)) close()
  }

  createEffect(() => {
    if (!isOpen()) return
    document.addEventListener('pointerdown', onPointerDown, true)
    document.addEventListener('focusin', onFocusIn, true)
    onCleanup(() => {
      document.removeEventListener('pointerdown', onPointerDown, true)
      document.removeEventListener('focusin', onFocusIn, true)
    })
  })
}
