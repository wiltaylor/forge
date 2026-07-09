import { describe, it, expect } from 'vitest';
import {
  cardsInColumn,
  columnIndexOf,
  moveCard,
  insertionIndex,
  type KanbanCard,
} from '../src/board';

const c = (id: string, column: string): KanbanCard => ({ id, column, data: {} });

// a1 a2 a3 in column a; b1 b2 in column b; column c empty
const board = [c('a1', 'a'), c('b1', 'b'), c('a2', 'a'), c('a3', 'a'), c('b2', 'b')];

const order = (cards: KanbanCard[], col: string) => cardsInColumn(cards, col).map((x) => x.id);

describe('cardsInColumn / columnIndexOf', () => {
  it('filters by column preserving flat order', () => {
    expect(order(board, 'a')).toEqual(['a1', 'a2', 'a3']);
    expect(order(board, 'b')).toEqual(['b1', 'b2']);
    expect(order(board, 'c')).toEqual([]);
  });
  it('columnIndexOf counts same-column cards before it', () => {
    expect(columnIndexOf(board, 'a1')).toBe(0);
    expect(columnIndexOf(board, 'a3')).toBe(2);
    expect(columnIndexOf(board, 'b2')).toBe(1);
    expect(columnIndexOf(board, 'nope')).toBe(-1);
  });
});

describe('moveCard — within a column', () => {
  it('moves a card down', () => {
    expect(order(moveCard(board, 'a1', 'a', 2), 'a')).toEqual(['a2', 'a3', 'a1']);
  });
  it('moves a card up to first', () => {
    expect(order(moveCard(board, 'a3', 'a', 0), 'a')).toEqual(['a3', 'a1', 'a2']);
  });
  it('index counts the column without the moved card', () => {
    // placing a2 at index 1 (of [a1, a3]) puts it between a1 and a3 — original spot
    expect(order(moveCard(board, 'a2', 'a', 1), 'a')).toEqual(['a1', 'a2', 'a3']);
  });
  it('keeps the moved card object identity when column unchanged', () => {
    const moved = moveCard(board, 'a1', 'a', 2);
    expect(moved.find((x) => x.id === 'a1')).toBe(board[0]);
  });
});

describe('moveCard — across columns', () => {
  it('moves into the middle of another column', () => {
    const moved = moveCard(board, 'a1', 'b', 1);
    expect(order(moved, 'b')).toEqual(['b1', 'a1', 'b2']);
    expect(order(moved, 'a')).toEqual(['a2', 'a3']);
  });
  it('appends at the end of another column', () => {
    expect(order(moveCard(board, 'a1', 'b', 2), 'b')).toEqual(['b1', 'b2', 'a1']);
  });
  it('moves into an empty column', () => {
    expect(order(moveCard(board, 'b2', 'c', 0), 'c')).toEqual(['b2']);
  });
  it('creates a new object only for the moved card, reusing all others', () => {
    const moved = moveCard(board, 'a1', 'b', 0);
    const movedCard = moved.find((x) => x.id === 'a1')!;
    expect(movedCard).not.toBe(board[0]);
    expect(movedCard.column).toBe('b');
    for (const orig of board.slice(1)) expect(moved).toContain(orig);
  });
  it('does not mutate its input', () => {
    moveCard(board, 'a1', 'b', 0);
    expect(board[0]!.column).toBe('a');
    expect(board).toHaveLength(5);
  });
});

describe('moveCard — clamping & misses', () => {
  it('clamps negative and past-end indices', () => {
    expect(order(moveCard(board, 'a1', 'b', -5), 'b')).toEqual(['a1', 'b1', 'b2']);
    expect(order(moveCard(board, 'a1', 'b', 99), 'b')).toEqual(['b1', 'b2', 'a1']);
  });
  it('unknown card id returns the input array', () => {
    expect(moveCard(board, 'nope', 'b', 0)).toBe(board);
  });
});

describe('insertionIndex', () => {
  const rects = [
    { top: 0, height: 40 },   // midpoint 20
    { top: 48, height: 40 },  // midpoint 68
    { top: 96, height: 40 },  // midpoint 116
  ];
  it('empty list → 0', () => {
    expect(insertionIndex(50, [])).toBe(0);
  });
  it('above all midpoints → 0', () => {
    expect(insertionIndex(10, rects)).toBe(0);
  });
  it('between midpoints → count above', () => {
    expect(insertionIndex(30, rects)).toBe(1);
    expect(insertionIndex(70, rects)).toBe(2);
  });
  it('below all midpoints → length', () => {
    expect(insertionIndex(200, rects)).toBe(3);
  });
});
