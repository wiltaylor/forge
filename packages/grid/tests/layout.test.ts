import { describe, it, expect } from 'vitest';
import {
  collides,
  boundsRows,
  resolvePush,
  compactUp,
  clampRect,
  previewLayout,
  type GridBlock,
} from '../src/layout';

const b = (id: string, x: number, y: number, w = 1, h = 1): GridBlock => ({ id, x, y, w, h });

const at = (layout: GridBlock[], id: string) => {
  const block = layout.find((blk) => blk.id === id);
  if (!block) throw new Error(`missing block ${id}`);
  return block;
};

const noOverlaps = (layout: GridBlock[]) =>
  layout.every((a, i) => layout.slice(i + 1).every((c) => !collides(a, c)));

describe('collides', () => {
  it('detects overlap', () => {
    expect(collides(b('a', 0, 0, 2, 2), b('b', 1, 1, 2, 2))).toBe(true);
  });
  it('edge-touching does not collide', () => {
    expect(collides(b('a', 0, 0, 2, 2), b('b', 2, 0, 1, 2))).toBe(false);
    expect(collides(b('a', 0, 0, 2, 2), b('b', 0, 2, 2, 1))).toBe(false);
  });
  it('a block never collides with itself', () => {
    expect(collides(b('a', 0, 0, 2, 2), b('a', 0, 0, 2, 2))).toBe(false);
  });
});

describe('boundsRows', () => {
  it('is max y+h, 0 when empty', () => {
    expect(boundsRows([])).toBe(0);
    expect(boundsRows([b('a', 0, 0, 1, 2), b('c', 3, 4, 1, 3)])).toBe(7);
  });
});

describe('resolvePush', () => {
  it('pushes a single overlapped block below the pinned one', () => {
    const out = resolvePush([b('pin', 0, 0, 2, 2), b('x', 0, 1, 1, 1)], 'pin');
    expect(at(out, 'pin')).toMatchObject({ x: 0, y: 0 });
    expect(at(out, 'x').y).toBe(2);
    expect(noOverlaps(out)).toBe(true);
  });

  it('chains pushes: pin pushes A pushes B', () => {
    const out = resolvePush(
      [b('pin', 0, 0, 1, 2), b('a', 0, 1, 1, 2), b('c', 0, 3, 1, 1)],
      'pin',
    );
    expect(at(out, 'a').y).toBe(2);
    expect(at(out, 'c').y).toBe(4);
    expect(noOverlaps(out)).toBe(true);
  });

  it('a wide pinned block pushes blocks in multiple columns', () => {
    const out = resolvePush(
      [b('pin', 0, 0, 3, 1), b('l', 0, 0, 1, 1), b('r', 2, 0, 1, 2)],
      'pin',
    );
    expect(at(out, 'l').y).toBe(1);
    expect(at(out, 'r').y).toBe(1);
    expect(noOverlaps(out)).toBe(true);
  });

  it('leaves non-colliding blocks untouched', () => {
    const out = resolvePush([b('pin', 0, 0, 1, 1), b('far', 3, 5, 2, 2)], 'pin');
    expect(at(out, 'far')).toMatchObject({ x: 3, y: 5 });
  });

  it('does not mutate its input', () => {
    const input = [b('pin', 0, 0, 2, 2), b('x', 0, 1, 1, 1)];
    resolvePush(input, 'pin');
    expect(input[1].y).toBe(1);
  });
});

describe('compactUp', () => {
  it('floats blocks up into gaps', () => {
    const out = compactUp([b('a', 0, 3, 1, 1), b('c', 2, 5, 1, 1)]);
    expect(at(out, 'a').y).toBe(0);
    expect(at(out, 'c').y).toBe(0);
  });

  it('stacks blocks in the same column without overlap', () => {
    const out = compactUp([b('a', 0, 2, 1, 2), b('c', 0, 6, 1, 1)]);
    expect(at(out, 'a').y).toBe(0);
    expect(at(out, 'c').y).toBe(2);
    expect(noOverlaps(out)).toBe(true);
  });

  it('a pinned block below a floater stops it', () => {
    const out = compactUp([b('pin', 0, 2, 1, 1), b('float', 0, 5, 1, 1)], 'pin');
    expect(at(out, 'pin').y).toBe(2);
    expect(at(out, 'float').y).toBe(3);
  });

  it('pinned block itself never moves', () => {
    const out = compactUp([b('pin', 1, 4, 1, 1)], 'pin');
    expect(at(out, 'pin').y).toBe(4);
  });
});

describe('clampRect', () => {
  it('clamps x into bounds and w to cols', () => {
    expect(clampRect({ x: -2, y: 0, w: 3, h: 1 }, 6)).toMatchObject({ x: 0, w: 3 });
    expect(clampRect({ x: 5, y: 0, w: 3, h: 1 }, 6)).toMatchObject({ x: 3, w: 3 });
    expect(clampRect({ x: 0, y: 0, w: 9, h: 1 }, 6)).toMatchObject({ x: 0, w: 6 });
  });
  it('enforces y >= 0 and w,h >= 1', () => {
    expect(clampRect({ x: 0, y: -3, w: 0, h: 0 }, 6)).toMatchObject({ y: 0, w: 1, h: 1 });
  });
});

describe('previewLayout', () => {
  const base = [b('a', 0, 0, 2, 2), b('c', 2, 0, 2, 2), b('d', 0, 2, 4, 1)];

  it('moving into an occupied cell pushes the occupant down and compacts', () => {
    const out = previewLayout(base, 'a', { x: 2, y: 0, w: 2, h: 2 }, 6);
    expect(at(out, 'a')).toMatchObject({ x: 2, y: 0 });
    expect(at(out, 'c').y).toBe(2);
    expect(noOverlaps(out)).toBe(true);
  });

  it('growing a block displaces its neighbour', () => {
    const out = previewLayout(base, 'a', { x: 0, y: 0, w: 3, h: 2 }, 6);
    expect(at(out, 'a')).toMatchObject({ w: 3 });
    expect(at(out, 'c').y).toBe(2);
    expect(noOverlaps(out)).toBe(true);
  });

  it('vacated space is reclaimed by compaction', () => {
    const out = previewLayout(base, 'a', { x: 0, y: 5, w: 2, h: 2 }, 6);
    expect(at(out, 'd').y).toBe(2);
    expect(at(out, 'a').y).toBe(5);
  });

  it('is idempotent for the same target', () => {
    const once = previewLayout(base, 'a', { x: 2, y: 0, w: 2, h: 2 }, 6);
    const twice = previewLayout(once, 'a', { x: 2, y: 0, w: 2, h: 2 }, 6);
    expect(twice).toEqual(once);
  });

  it('preserves extra consumer fields on the moved block', () => {
    const withLabel = [{ ...b('a', 0, 0, 2, 2), label: 'CPU' } as GridBlock, b('c', 2, 0, 2, 2)];
    const out = previewLayout(withLabel, 'a', { x: 2, y: 0, w: 2, h: 2 }, 6);
    expect((at(out, 'a') as GridBlock & { label?: string }).label).toBe('CPU');
  });

  it('handles an incoming block id not present in base (palette drop)', () => {
    const out = previewLayout(base, '__incoming__', { x: 0, y: 0, w: 2, h: 1 }, 6);
    expect(at(out, '__incoming__')).toMatchObject({ x: 0, y: 0 });
    expect(out).toHaveLength(4);
    expect(noOverlaps(out)).toBe(true);
  });
});
