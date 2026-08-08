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
  /**
   * Trap Tab and Shift-Tab inside the surface. A modal surface passes true, so
   * the keyboard cannot walk into the page behind it. Anchored surfaces leave
   * it unset.
   */
  trap?: boolean;
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
  if (e.key === 'Escape') stack[stack.length - 1]?.onDismiss();
  if (e.key === 'Tab') trapTab(e);
};

/* What Tab can reach. Order in the document is order in the cycle; the kit
   assigns no positive tabindex, so document order is tab order. */
const FOCUSABLE =
  'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), ' +
  'textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

/* The element with focus, resolved through shadow boundaries for the same
   reason pathOf uses composedPath: inside a shadow root the document reports
   the host, not the focused element. */
const focused = (): Element | null => {
  let el = document.activeElement;
  while (el?.shadowRoot?.activeElement) el = el.shadowRoot.activeElement;
  return el;
};

/* Keep Tab inside the innermost trapping surface. Only the edges are
   intercepted — from the last focusable forward to the first, from the first
   backward to the last, and from outside the surface back in — so movement in
   the middle keeps the browser's own tab order. An overlay anchored inside the
   surface counts as inside, so a popover in a modal does not trigger the
   wrap. */
const trapTab = (e: KeyboardEvent): void => {
  const layer = [...stack].reverse().find((l) => l.trap);
  const surface = layer?.surface();
  if (!surface) return;
  const focusables = Array.from(surface.querySelectorAll<HTMLElement>(FOCUSABLE));
  const first = focusables[0];
  const last = focusables[focusables.length - 1];
  if (!first || !last) {
    e.preventDefault();
    return;
  }
  const current = focused();
  const outside = !current || !surface.contains(current);
  if (!outside && current !== (e.shiftKey ? first : last)) return;
  e.preventDefault();
  (e.shiftKey ? last : first).focus();
};

/* Return focus to whatever opened the overlay, so a keyboard user does not
   lose their place. Only when focus is still in the overlay or fell to the
   body when the surface unmounted — a pointer that dismissed the overlay by
   landing elsewhere takes focus with it, and that is not ours to take back. */
function restore(layer: Layer, opener: Element | null): void {
  if (!(opener instanceof HTMLElement) || !opener.isConnected) return;
  const current = focused();
  const surface = layer.surface();
  const lost = !current || current === document.body;
  if (lost || (surface && current && surface.contains(current))) opener.focus();
}

function register(layer: Layer): () => void {
  const opener = focused();
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
    restore(layer, opener);
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
