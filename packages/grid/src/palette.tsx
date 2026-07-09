/* Palette drag-in. setPointerCapture keeps every pointer event on the
   PaletteItem, so the grid never sees the drag directly — instead the item
   publishes client coordinates through GridDndContext and each BlockGrid
   under the same GridDndProvider hit-tests them against its own rect.

   <GridDndProvider>
     <GridPalette>
       <PaletteItem template={{ w: 2, h: 1, label: 'Chart' }}>2x1 Chart</PaletteItem>
     </GridPalette>
     <BlockGrid ... onBlockAdd={(pos, template) => append block} />
   </GridDndProvider>

   Imports only solid-js — no coupling to @forge/ui. */

import { Show, createContext, createSignal, useContext } from 'solid-js';
import type { Accessor, JSX } from 'solid-js';
import { Portal } from 'solid-js/web';

/** Block blueprint carried by a palette drag; w/h in grid units. Extra fields
    pass through untouched to BlockGrid's onBlockAdd. */
export interface PaletteTemplate {
  w: number;
  h: number;
  [key: string]: unknown;
}

export interface PaletteDragState {
  template: PaletteTemplate;
  /** Pointer position in client (viewport) coordinates. */
  x: number;
  y: number;
  phase: 'move' | 'drop';
}

export interface GridDndValue {
  drag: Accessor<PaletteDragState | null>;
  setDrag: (d: PaletteDragState | null) => void;
}

export const GridDndContext = createContext<GridDndValue>();

export function GridDndProvider(props: { children: JSX.Element }) {
  const [drag, setDrag] = createSignal<PaletteDragState | null>(null);
  return (
    <GridDndContext.Provider value={{ drag, setDrag }}>{props.children}</GridDndContext.Provider>
  );
}

export function GridPalette(props: { class?: string; children: JSX.Element }) {
  return <div class={`fpalette ${props.class ?? ''}`}>{props.children}</div>;
}

/* Nominal cell size used only for the floating ghost — the real cell size
   belongs to whichever grid the drop lands on. */
const GHOST_CELL = 48;
const GHOST_GAP = 8;

export function PaletteItem(props: { template: PaletteTemplate; children?: JSX.Element }) {
  const dnd = useContext(GridDndContext);
  const [ghost, setGhost] = createSignal<{ x: number; y: number } | null>(null);
  let start: { x: number; y: number } | null = null;

  const ghostW = () => props.template.w * GHOST_CELL + (props.template.w - 1) * GHOST_GAP;
  const ghostH = () => props.template.h * GHOST_CELL + (props.template.h - 1) * GHOST_GAP;

  const down = (e: PointerEvent) => {
    if (!dnd || e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    start = { x: e.clientX, y: e.clientY };
  };
  const move = (e: PointerEvent) => {
    if (!dnd || !start) return;
    // 4px threshold before the drag activates, so a plain click stays a click.
    if (!ghost() && Math.hypot(e.clientX - start.x, e.clientY - start.y) < 4) return;
    setGhost({ x: e.clientX, y: e.clientY });
    dnd.setDrag({ template: props.template, x: e.clientX, y: e.clientY, phase: 'move' });
  };
  const up = (e: PointerEvent) => {
    if (dnd && start && ghost()) {
      dnd.setDrag({ template: props.template, x: e.clientX, y: e.clientY, phase: 'drop' });
      dnd.setDrag(null);
    }
    start = null;
    setGhost(null);
  };

  return (
    <div
      class="fpalette-item"
      classList={{ 'is-dragging': !!ghost() }}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
    >
      {props.children}
      <Show when={ghost()}>
        {(g) => (
          <Portal>
            <div
              class="fpalette-ghost"
              style={{
                left: `${g().x - ghostW() / 2}px`,
                top: `${g().y - ghostH() / 2}px`,
                width: `${ghostW()}px`,
                height: `${ghostH()}px`,
              }}
            >
              {props.children}
            </div>
          </Portal>
        )}
      </Show>
    </div>
  );
}
