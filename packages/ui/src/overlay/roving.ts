/* The roving index: which item of a listbox-shaped widget is the active one,
   and what the arrow keys, Home and End do to it. Menus, selects, listboxes,
   comboboxes and the command palette all adapt this one implementation.

   Movement wraps. That is the kit's single policy: ArrowDown on the last item
   goes to the first, ArrowUp on the first goes to the last, in every widget.
   Home and End are the way to reach an end deliberately, so wrapping costs a
   keyboard user nothing and arrowing behaves the same wherever they are.

   The active item is an index, not focus. Focus stays on the trigger or the
   search input while the index moves, which is what lets a Select popup and a
   Combobox list be driven from a control outside them. */

import { createSignal } from 'solid-js';
import type { Accessor } from 'solid-js';

/** The index when no item is active. */
export const NO_ACTIVE_INDEX = -1;

export interface RovingOptions {
  /** How many items the widget holds now. It is read on every move. */
  count: () => number;
  /**
   * Can the item at this index take the active position? Absent means every
   * item can. A disabled option and a menu separator answer false, and
   * movement passes over them.
   */
  enabled?: (index: number) => boolean;
}

export interface Roving {
  /** The active item, or `NO_ACTIVE_INDEX` when there is none. */
  active: Accessor<number>;
  /** Put the index on an item — a pointer entering it, for one. */
  setActive: (index: number) => void;
  /** Leave no item active. */
  clear: () => void;
  /** Move to the first item that can take the index. */
  first: () => void;
  /** Move to the last one. */
  last: () => void;
  /**
   * Handle a movement key. It returns true when the key moved the index, and
   * has called `preventDefault`; the caller stops there. False leaves the key
   * to the caller — Enter, Escape, typing and the rest are not this module's.
   */
  onKeyDown: (e: KeyboardEvent) => boolean;
}

/** Give a widget the kit's roving index. */
export function createRoving(options: RovingOptions): Roving {
  const [active, setIndex] = createSignal(NO_ACTIVE_INDEX);
  const canTake = (i: number): boolean => options.enabled?.(i) ?? true;

  /* Scan from `from` in `dir` for an item that can take the index, wrapping at
     both ends. It looks at each item once, so a widget whose items are all
     disabled answers "nowhere to go" rather than spinning. */
  const scan = (from: number, dir: number): number => {
    const n = options.count();
    if (n <= 0) return NO_ACTIVE_INDEX;
    let i = ((from % n) + n) % n;
    for (let step = 0; step < n; step++) {
      if (canTake(i)) return i;
      i = (i + dir + n) % n;
    }
    return NO_ACTIVE_INDEX;
  };

  /* One rule for every move: the index goes to the first item the scan finds,
     and where it finds none — an empty list, or one whose items are all
     disabled — no item is active. */
  const move = (dir: number): void => {
    const n = options.count();
    /* From nowhere, a step forward lands on the first item and a step back on
       the last — the same items wrapping would reach. */
    const from = active() === NO_ACTIVE_INDEX ? (dir > 0 ? 0 : n - 1) : active() + dir;
    setIndex(scan(from, dir));
  };
  const first = (): void => { setIndex(scan(0, 1)); };
  const last = (): void => { setIndex(scan(options.count() - 1, -1)); };

  const onKeyDown = (e: KeyboardEvent): boolean => {
    if (e.key === 'ArrowDown') move(1);
    else if (e.key === 'ArrowUp') move(-1);
    else if (e.key === 'Home') first();
    else if (e.key === 'End') last();
    else return false;
    e.preventDefault();
    return true;
  };

  return {
    active,
    setActive: (index) => { setIndex(index); },
    clear: () => { setIndex(NO_ACTIVE_INDEX); },
    first,
    last,
    onKeyDown,
  };
}
