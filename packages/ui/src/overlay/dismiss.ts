/* Dismissal for every overlay in the kit: outside interaction, Escape, and the
   precedence between overlays that are open at the same time.

   One document listener of each kind serves every open overlay, because
   precedence is a property of the set and not of any one member. Overlays
   register in the order they open, so the innermost — the one opened last — is
   the end of the stack. */

import { createEffect, onCleanup } from 'solid-js';

export interface OverlayOptions {
  /** Is the overlay open? It registers while this is true and not before. */
  open: () => boolean;
  /**
   * The overlay's own element. A pointer landing in it, or in the surface of an
   * overlay opened after it, is not an outside interaction.
   */
  surface: () => Element | undefined | null;
  /**
   * The element an outside pointer must land in for it to dismiss. Absent — the
   * default for anchored surfaces — means anywhere in the document. A modal
   * passes its backdrop, so that what is painted above the backdrop, a toast
   * for one, does not dismiss the surface underneath it.
   */
  backdrop?: () => Element | undefined | null;
  /** Close the overlay. It stays open until its owner acts on this. */
  onDismiss: () => void;
}

/* An open overlay. `open` is the registration condition, so the stack does not
   carry it. */
type Layer = Omit<OverlayOptions, 'open'>;

/* Open overlays, innermost last. */
const stack: Layer[] = [];

/* composedPath() reports the path through a shadow boundary, which `contains`
   cannot: the mount seam puts overlays inside a shadow root, and there the
   event target the document sees is the host element. */
const pathOf = (e: Event): readonly EventTarget[] =>
  e.composedPath ? e.composedPath() : e.target ? [e.target] : [];

const onPointerDown = (e: Event): void => {
  const path = pathOf(e);
  /* Innermost first, and stop at the first overlay the pointer landed in: a
     click on a menu that opened inside a modal must not close the modal. */
  for (const layer of [...stack].reverse()) {
    const surface = layer.surface();
    if (surface && path.includes(surface)) break;
    const backdrop = layer.backdrop?.();
    if (backdrop && !path.includes(backdrop)) continue;
    layer.onDismiss();
  }
};

const onKeyDown = (e: KeyboardEvent): void => {
  if (e.key !== 'Escape') return;
  stack[stack.length - 1]?.onDismiss();
};

function register(layer: Layer): () => void {
  stack.push(layer);
  if (stack.length === 1) {
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
  }
  return () => {
    const i = stack.indexOf(layer);
    if (i >= 0) stack.splice(i, 1);
    if (stack.length === 0) {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    }
  };
}

/**
 * Give an overlay the kit's dismissal behaviour. It closes when a pointer lands
 * outside it, and on Escape while it is the innermost open overlay.
 *
 * This is the seam every overlay in the kit adapts through, in @forge/ui and in
 * the packages built on it.
 */
export function useOverlay(options: OverlayOptions): void {
  createEffect(() => {
    if (!options.open()) return;
    onCleanup(register(options));
  });
}
