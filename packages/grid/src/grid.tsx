/* Forge block grid — a gridstack-style draggable dashboard grid over the
   grid.css class layer. Controlled component: the layout comes from props;
   during a move/resize/palette drag the grid renders a private preview (push +
   compact via layout.ts) and commits it through onLayoutChange ONCE on drop —
   never per pointermove.

   <BlockGrid
     layout={[{ id, x, y, w, h }]}     // grid units
     cols={12} rowHeight={80} gap={8}
     onLayoutChange={(layout) => ...}  // once per completed drag; preserve
                                       // untouched block objects when storing
     onBlockAdd={(pos, template) => ...} // palette drop landed; mint id + append
   >
     {(block) => <block body JSX>}
   </BlockGrid>

   Columns are fluid (measured via ResizeObserver); rows are fixed-px.
   Imports only solid-js — no coupling to @forge/ui. */

import { For, Show, createEffect, createMemo, createSignal, on, onCleanup, onMount, useContext } from 'solid-js';
import type { JSX } from 'solid-js';
import { boundsRows, previewLayout } from './layout';
import type { GridBlock, GridRect } from './layout';
import { GridDndContext } from './palette';
import type { PaletteTemplate } from './palette';

export interface BlockGridProps {
  layout: GridBlock[];
  /** Column count; columns stretch to the container width. Default 12. */
  cols?: number;
  /** Row height in px. Default 80. */
  rowHeight?: number;
  /** Gap between cells in px, both axes. Default 8. */
  gap?: number;
  /** Minimum canvas height in rows. Default 4. */
  minRows?: number;
  /** Committed layout after a completed move/resize. Fires once on drop. */
  onLayoutChange?: (layout: GridBlock[]) => void;
  /** A palette drop landed on this grid; the consumer mints the id and appends. */
  onBlockAdd?: (pos: GridRect, template: PaletteTemplate) => void;
  class?: string;
  style?: JSX.CSSProperties | string;
  /** Block body render prop. */
  children?: (block: GridBlock) => JSX.Element;
}

/* Id used to pin an in-flight palette block; never appears in props.layout. */
const INCOMING = '__incoming__';

/* Elements a move-drag must not start from, so block content stays usable. */
const NO_DRAG = 'button, a, input, select, textarea, [data-no-drag]';

interface DragState {
  id: string;
  kind: 'move' | 'resize';
  /** Pointer offset from the block's top-left (move) / bottom-right (resize), canvas px. */
  dx: number;
  dy: number;
  start: GridBlock;
  active: boolean;
}

export function BlockGrid(props: BlockGridProps) {
  let root!: HTMLDivElement;
  let drag: DragState | null = null; // plain variable, not reactive (per-frame)

  const cols = () => props.cols ?? 12;
  const rowH = () => props.rowHeight ?? 80;
  const gap = () => props.gap ?? 8;

  const [containerW, setContainerW] = createSignal(0);
  /* Reactive drag state — only what actually renders. */
  const [preview, setPreview] = createSignal<GridBlock[] | null>(null);
  const [activeId, setActiveId] = createSignal<string | null>(null);
  const [dragPx, setDragPx] = createSignal<{ kind: 'move' | 'resize'; left: number; top: number; width: number; height: number } | null>(null);

  onMount(() => {
    const ro = new ResizeObserver(() => setContainerW(root.clientWidth));
    ro.observe(root);
    onCleanup(() => ro.disconnect());
  });

  const cellW = createMemo(() => (containerW() - (cols() - 1) * gap()) / cols());
  const colPx = (x: number) => x * (cellW() + gap());
  const rowPx = (y: number) => y * (rowH() + gap());
  const wPx = (w: number) => w * cellW() + (w - 1) * gap();
  const hPx = (h: number) => h * rowH() + (h - 1) * gap();

  const display = () => preview() ?? props.layout;
  /* Preview arrays are fresh objects every pointermove; blocks render from the
     stable props.layout and look their live position up here, so DOM is never
     recreated mid-drag. */
  const posMap = createMemo(() => {
    const m = new Map<string, GridBlock>();
    for (const b of display()) m.set(b.id, b);
    return m;
  });

  const placeholder = createMemo(() => {
    const id = activeId();
    return id ? (posMap().get(id) ?? null) : null;
  });

  const rows = createMemo(() =>
    Math.max(props.minRows ?? 4, boundsRows(display())) + (activeId() ? 1 : 0));

  const rootStyle = (): JSX.CSSProperties | string => {
    const minH = `${rows() * (rowH() + gap()) - gap()}px`;
    if (typeof props.style === 'string') return `${props.style};min-height:${minH}`;
    return { ...props.style, 'min-height': minH };
  };

  const toCanvas = (e: PointerEvent) => {
    const r = root.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  };
  const snap = (px: number, unit: number) => Math.round(px / (unit + gap()));

  const runPreview = (target: GridRect, id: string) => {
    setPreview(previewLayout(props.layout, id, target, cols()));
    setActiveId(id);
  };

  /* ---- move / resize (blocks already on the grid) ------------------------- */

  const blockDown = (block: GridBlock) => (e: PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest(NO_DRAG)) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const p = toCanvas(e);
    drag = { id: block.id, kind: 'move', dx: p.x - colPx(block.x), dy: p.y - rowPx(block.y), start: { ...block }, active: false };
  };

  const resizeDown = (block: GridBlock) => (e: PointerEvent) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const p = toCanvas(e);
    drag = {
      id: block.id,
      kind: 'resize',
      dx: p.x - (colPx(block.x) + wPx(block.w)),
      dy: p.y - (rowPx(block.y) + hPx(block.h)),
      start: { ...block },
      active: false,
    };
  };

  const rootMove = (e: PointerEvent) => {
    if (!drag) return;
    const p = toCanvas(e);
    if (!drag.active) {
      const { start } = drag;
      const grabX = drag.kind === 'move' ? colPx(start.x) + drag.dx : colPx(start.x) + wPx(start.w) + drag.dx;
      const grabY = drag.kind === 'move' ? rowPx(start.y) + drag.dy : rowPx(start.y) + hPx(start.h) + drag.dy;
      if (Math.hypot(p.x - grabX, p.y - grabY) < 4) return; // click passthrough
      drag.active = true;
    }
    const { id, kind, dx, dy, start } = drag;
    if (kind === 'move') {
      const left = p.x - dx;
      const top = p.y - dy;
      setDragPx({ kind, left, top, width: wPx(start.w), height: hPx(start.h) });
      runPreview({ x: snap(left, cellW()), y: snap(top, rowH()), w: start.w, h: start.h }, id);
    } else {
      const width = Math.max(cellW() / 2, p.x - dx - colPx(start.x));
      const height = Math.max(rowH() / 2, p.y - dy - rowPx(start.y));
      setDragPx({ kind, left: colPx(start.x), top: rowPx(start.y), width, height });
      runPreview(
        { x: start.x, y: start.y, w: Math.max(1, snap(width + gap(), cellW())), h: Math.max(1, snap(height + gap(), rowH())) },
        id,
      );
    }
  };

  const rootUp = () => {
    if (!drag) return;
    const committed = drag.active ? preview() : null;
    drag = null;
    setActiveId(null);
    setPreview(null);
    setDragPx(null);
    if (committed) props.onLayoutChange?.(committed);
  };

  /* ---- palette drag-in ----------------------------------------------------- */

  const dnd = useContext(GridDndContext);
  if (dnd) {
    createEffect(on(dnd.drag, (d) => {
      if (drag) return; // internal drag wins
      if (!d) {
        if (activeId() === INCOMING) { setActiveId(null); setPreview(null); }
        return;
      }
      const r = root.getBoundingClientRect();
      const inside = d.x >= r.left && d.x <= r.right && d.y >= r.top && d.y <= r.bottom;
      if (!inside) {
        if (activeId() === INCOMING) { setActiveId(null); setPreview(null); }
        return;
      }
      const { w, h } = d.template;
      const target: GridRect = {
        x: snap(d.x - r.left - wPx(w) / 2, cellW()),
        y: snap(d.y - r.top - hPx(h) / 2, rowH()),
        w,
        h,
      };
      if (d.phase === 'move') {
        runPreview(target, INCOMING);
        return;
      }
      // drop: commit the pushed positions, then hand the landing rect to the consumer.
      const landed = previewLayout(props.layout, INCOMING, target, cols());
      const pos = landed.find((b) => b.id === INCOMING)!;
      setActiveId(null);
      setPreview(null);
      props.onLayoutChange?.(landed.filter((b) => b.id !== INCOMING));
      props.onBlockAdd?.({ x: pos.x, y: pos.y, w: pos.w, h: pos.h }, d.template);
    }));
  }

  /* ---- render --------------------------------------------------------------- */

  const blockStyle = (block: GridBlock): JSX.CSSProperties => {
    const px = dragPx();
    if (px && activeId() === block.id) {
      return { left: `${px.left}px`, top: `${px.top}px`, width: `${px.width}px`, height: `${px.height}px` };
    }
    const pos = posMap().get(block.id) ?? block;
    return { left: `${colPx(pos.x)}px`, top: `${rowPx(pos.y)}px`, width: `${wPx(pos.w)}px`, height: `${hPx(pos.h)}px` };
  };

  return (
    <div
      ref={root}
      class={`fblockgrid ${props.class ?? ''}`}
      style={rootStyle()}
      onPointerMove={rootMove}
      onPointerUp={rootUp}
      onPointerCancel={rootUp}
    >
      <Show when={cellW() > 0}>
        <Show when={placeholder()}>
          {(ph) => (
            <div
              class="fblockgrid-placeholder"
              style={{ left: `${colPx(ph().x)}px`, top: `${rowPx(ph().y)}px`, width: `${wPx(ph().w)}px`, height: `${hPx(ph().h)}px` }}
            />
          )}
        </Show>
        <For each={props.layout}>
          {(block) => (
            <div
              class="fblockgrid-block"
              classList={{
                'is-dragging': activeId() === block.id && dragPx()?.kind === 'move',
                'is-resizing': activeId() === block.id && dragPx()?.kind === 'resize',
              }}
              style={blockStyle(block)}
              onPointerDown={blockDown(block)}
            >
              {props.children?.(block)}
              <div class="fblockgrid-resize" onPointerDown={resizeDown(block)} />
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
