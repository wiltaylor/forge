/* Pure grid-layout engine — no DOM, no solid-js. Blocks live in integer grid
   units; the component converts to pixels. All functions return fresh objects
   and never mutate their inputs. */

export interface GridBlock {
  id: string;
  /** Column, in grid units. */
  x: number;
  /** Row, in grid units. */
  y: number;
  /** Width in columns. */
  w: number;
  /** Height in rows. */
  h: number;
}

/** Target rect for a move/resize/drop, in grid units. */
export interface GridRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** AABB overlap. Edge-touching blocks do NOT collide. */
export const collides = (a: GridBlock, b: GridBlock): boolean =>
  a.id !== b.id && a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;

/** Rows spanned by the layout (max y + h). */
export const boundsRows = (blocks: GridBlock[]): number =>
  blocks.reduce((max, b) => Math.max(max, b.y + b.h), 0);

const byPosition = (a: GridBlock, b: GridBlock) => a.y - b.y || a.x - b.x;

/* Push pass: the pinned block keeps its position; every other block, taken in
   (y, x) order, is bumped downward until it overlaps nothing already placed.
   y only ever grows, so this terminates, and chained pushes (A pushes B pushes
   C) fall out of the ordering — a bumped block lands before the blocks below
   it are processed. */
export function resolvePush(blocks: GridBlock[], pinnedId: string): GridBlock[] {
  const pinned = blocks.find((b) => b.id === pinnedId);
  const rest = blocks
    .filter((b) => b.id !== pinnedId)
    .map((b) => ({ ...b }))
    .sort(byPosition);
  const placed: GridBlock[] = pinned ? [{ ...pinned }] : [];
  for (const block of rest) {
    let hit: GridBlock | undefined;
    while ((hit = placed.find((p) => collides(block, p)))) {
      block.y = hit.y + hit.h;
    }
    placed.push(block);
  }
  return placed;
}

/* Float pass: every non-pinned block, in (y, x) order, rises one row at a time
   until it would overlap something already placed (or hits row 0). Row-by-row
   rather than a closed form so a pinned block sitting BELOW a floater still
   stops it. */
export function compactUp(blocks: GridBlock[], pinnedId?: string): GridBlock[] {
  const sorted = blocks.map((b) => ({ ...b })).sort(byPosition);
  const placed: GridBlock[] = [];
  const pinned = sorted.find((b) => b.id === pinnedId);
  if (pinned) placed.push(pinned);
  for (const block of sorted) {
    if (block === pinned) continue;
    while (block.y > 0) {
      block.y -= 1;
      if (placed.some((p) => collides(block, p))) {
        block.y += 1;
        break;
      }
    }
    placed.push(block);
  }
  return placed;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

/** Clamp a target rect into the grid: w,h >= 1, w <= cols, fully in-bounds. */
export function clampRect(target: GridRect, cols: number): GridRect {
  const w = clamp(Math.round(target.w), 1, cols);
  const h = Math.max(1, Math.round(target.h));
  return {
    w,
    h,
    x: clamp(Math.round(target.x), 0, cols - w),
    y: Math.max(0, Math.round(target.y)),
  };
}

/* The pipeline move, resize, and palette drag-over all run per pointermove:
   pin the moving block at the (clamped) target, push everything else out of
   the way, then float the rest back up into any gaps. The result is what the
   grid renders as a live preview and commits verbatim on drop — the
   placeholder is the truth, no post-drop jump. (Compacting the pinned block
   too on drop would give stricter gridstack parity; deliberate non-goal.) */
export function previewLayout(
  base: GridBlock[],
  movingId: string,
  target: GridRect,
  cols: number,
): GridBlock[] {
  const rect = clampRect(target, cols);
  // Spread the original first: consumers may hang extra fields (label, kind…)
  // off their blocks, and those must survive a move/resize round-trip.
  const orig = base.find((b) => b.id === movingId);
  const pinned: GridBlock = { ...orig, id: movingId, ...rect };
  const others = base.filter((b) => b.id !== movingId);
  return compactUp(resolvePush([pinned, ...others], movingId), movingId);
}
