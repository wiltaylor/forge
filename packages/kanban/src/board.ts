/* Pure kanban board logic — no DOM, no solid-js (types from @forge/ui only).
   Cards live in ONE flat array; a card's column is `card.column` and its order
   within that column is its relative order in the array. All functions return
   fresh arrays but reuse untouched card objects, so Solid <For> keeps DOM. */

import type { Option, Tone } from '@forge/ui';

export interface KanbanColumn {
  id: string;
  title: string;
  collapsed?: boolean;
}

/** One control on a card, keyed into `card.data`. */
export type KanbanField = { key: string; label?: string } & (
  | { type: 'text'; placeholder?: string }
  | { type: 'textarea'; rows?: number }
  | { type: 'select'; options: Option<string>[]; placeholder?: string }
  | { type: 'date'; min?: string; max?: string }
  | { type: 'checkbox' }
  | { type: 'toggle' }
  | { type: 'slider'; min?: number; max?: number; step?: number; showValue?: boolean }
  /** Display-only badge; tone looked up by String(value). */
  | { type: 'badge'; tones?: Record<string, Tone> }
);

export interface KanbanCard {
  id: string;
  /** KanbanColumn id. */
  column: string;
  title?: string;
  data: Record<string, unknown>;
  /** Per-card schema override; falls back to the board-level schema. */
  fields?: KanbanField[];
}

export const cardsInColumn = (cards: KanbanCard[], columnId: string): KanbanCard[] =>
  cards.filter((c) => c.column === columnId);

/** Position of `cardId` within its own column, counting other cards only. */
export const columnIndexOf = (cards: KanbanCard[], cardId: string): number => {
  const card = cards.find((c) => c.id === cardId);
  if (!card) return -1;
  return cards.filter((c) => c.column === card.column && c.id !== cardId && cards.indexOf(c) < cards.indexOf(card)).length;
};

/* Move a card to `index` within `toColumn` (index counted over that column's
   cards EXCLUDING the moved one — matching what a drag placeholder shows).
   Returns a fresh array; the moved card keeps its object identity unless its
   column changes, and every other card is reused as-is. */
export function moveCard(
  cards: KanbanCard[],
  cardId: string,
  toColumn: string,
  index: number,
): KanbanCard[] {
  const card = cards.find((c) => c.id === cardId);
  if (!card) return cards;
  const rest = cards.filter((c) => c.id !== cardId);
  const colCards = rest.filter((c) => c.column === toColumn);
  const i = Math.max(0, Math.min(Math.round(index), colCards.length));
  const flatPos =
    i < colCards.length
      ? rest.indexOf(colCards[i]!)
      : colCards.length > 0
        ? rest.indexOf(colCards[colCards.length - 1]!) + 1
        : rest.length;
  const moved = card.column === toColumn ? card : { ...card, column: toColumn };
  return [...rest.slice(0, flatPos), moved, ...rest.slice(flatPos)];
}

/* Insertion index from a pointer's Y position over a stack of card rects:
   the number of cards whose vertical midpoint sits above the pointer. */
export const insertionIndex = (
  pointerY: number,
  rects: { top: number; height: number }[],
): number => rects.filter((r) => r.top + r.height / 2 < pointerY).length;
