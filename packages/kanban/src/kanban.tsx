/* Forge kanban board — controlled component over the kanban.css class layer.
   Columns and cards come from props (cards as ONE flat array, order within a
   column = relative order in the array); interactions are reported through
   callbacks and the consumer updates its own store:

   <KanbanBoard
     columns={[{ id, title, collapsed? }]}
     cards={[{ id, column, title?, data, fields? }]}
     fields={[{ key, label?, type: 'text'|'select'|'date'|..., ... }]}
     onCardsChange={(cards) => ...}      // fires ONCE per completed drag
     onCardChange={(id, data) => ...}    // per control edit (full next data)
     onCardAdd={(columnId) => ...}       // consumer mints the id + appends
     onCardRemove={(cardId) => ...}
     onColumnToggle={(columnId, collapsed) => ...}
   >
     {(card) => <optional extra JSX below the fields>}
   </KanbanBoard>

   Dropping onto a collapsed column appends to its end. Store the array from
   onCardsChange verbatim — untouched card objects are reused so <For> keeps
   their DOM. */

import { For, Show, createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { Badge, IconButton } from '@forge/ui';
import { cardsInColumn, columnIndexOf, insertionIndex, moveCard } from './board';
import type { KanbanCard, KanbanColumn, KanbanField } from './board';
import { CardFields } from './fields';

export interface KanbanBoardProps {
  columns: KanbanColumn[];
  cards: KanbanCard[];
  /** Board-level schema; card.fields overrides per card. */
  fields?: KanbanField[];
  /** Committed cards array after a completed drag. Fires once on drop. */
  onCardsChange?: (cards: KanbanCard[]) => void;
  /** A card control was edited; `data` is the full next data object. */
  onCardChange?: (cardId: string, data: Record<string, unknown>) => void;
  /** The column's add button was pressed; the consumer mints id + appends. */
  onCardAdd?: (columnId: string) => void;
  onCardRemove?: (cardId: string) => void;
  onColumnToggle?: (columnId: string, collapsed: boolean) => void;
  class?: string;
  style?: JSX.CSSProperties | string;
  /** Extra content rendered below a card's fields. */
  children?: (card: KanbanCard) => JSX.Element;
}

/* Wider than grid's: label (Checkbox/Toggle wrap in <label>) and the inline
   Select/DatePicker popovers, so clicks inside them never start a card drag. */
const NO_DRAG = 'button, a, input, select, textarea, label, [data-no-drag], .fselect-pop, .fpop';

const EDGE = 28;   // px from a column body's edge that triggers auto-scroll
const SCROLL = 10; // px per frame

interface DragState {
  cardId: string;
  fromColumn: string;
  startX: number;
  startY: number;
  offsetX: number;
  offsetY: number;
  width: number;
  height: number;
  active: boolean;
}

const PlusSvg = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M8 3v10M3 8h10" />
  </svg>
);
const ChevronSvg = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M10 4l-4 4 4 4" />
  </svg>
);
const XSvg = () => (
  <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M4 4l8 8M12 4l-8 8" />
  </svg>
);

export function KanbanBoard(props: KanbanBoardProps) {
  let root!: HTMLDivElement;
  let drag: DragState | null = null; // plain variable, not reactive (per-frame)
  let raf = 0;
  let lastPointer = { x: 0, y: 0 };

  /* Reactive drag state — only what renders. */
  const [dragId, setDragId] = createSignal<string | null>(null);
  const [dragPos, setDragPos] = createSignal({ left: 0, top: 0 });
  const [target, setTarget] = createSignal<{ column: string; index: number } | null>(null);

  const fieldsFor = (card: KanbanCard) => card.fields ?? props.fields ?? [];

  /* ---- drag & drop ---------------------------------------------------------- */

  const cardDown = (card: KanbanCard) => (e: PointerEvent) => {
    if (e.button !== 0 || (e.target as Element).closest(NO_DRAG)) return;
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const r = el.getBoundingClientRect();
    drag = {
      cardId: card.id,
      fromColumn: card.column,
      startX: e.clientX,
      startY: e.clientY,
      offsetX: e.clientX - r.left,
      offsetY: e.clientY - r.top,
      width: r.width,
      height: r.height,
      active: false,
    };
    lastPointer = { x: e.clientX, y: e.clientY };
  };

  /* Live rects every move — column scroll or layout shifts can't go stale. */
  const hitTest = (x: number, y: number) => {
    let best: { column: string; index: number } | null = null;
    let bestDist = Infinity;
    for (const colEl of root.querySelectorAll<HTMLElement>('[data-col]')) {
      const r = colEl.getBoundingClientRect();
      const dist = x < r.left ? r.left - x : x > r.right ? x - r.right : 0;
      if (dist >= bestDist) continue;
      bestDist = dist;
      const id = colEl.dataset.col!;
      const col = props.columns.find((c) => c.id === id);
      if (col?.collapsed) {
        best = { column: id, index: cardsInColumn(props.cards, id).length };
        continue;
      }
      const rects = [...colEl.querySelectorAll<HTMLElement>('[data-card]')]
        .filter((cardEl) => cardEl.dataset.card !== drag!.cardId)
        .map((cardEl) => cardEl.getBoundingClientRect());
      best = { column: id, index: insertionIndex(y, rects) };
    }
    return best;
  };

  const autoScroll = () => {
    if (!drag?.active) return;
    const body = root.querySelector<HTMLElement>(`[data-col-body="${target()?.column}"]`);
    if (body) {
      const r = body.getBoundingClientRect();
      if (lastPointer.y < r.top + EDGE) body.scrollTop -= SCROLL;
      else if (lastPointer.y > r.bottom - EDGE) body.scrollTop += SCROLL;
      setTarget(hitTest(lastPointer.x, lastPointer.y));
    }
    raf = requestAnimationFrame(autoScroll);
  };

  const rootMove = (e: PointerEvent) => {
    if (!drag) return;
    lastPointer = { x: e.clientX, y: e.clientY };
    if (!drag.active) {
      if (Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) < 4) return; // click passthrough
      drag.active = true;
      setDragId(drag.cardId);
      raf = requestAnimationFrame(autoScroll);
    }
    setDragPos({ left: e.clientX - drag.offsetX, top: e.clientY - drag.offsetY });
    setTarget(hitTest(e.clientX, e.clientY));
  };

  const rootUp = () => {
    if (!drag) return;
    const t = drag.active ? target() : null;
    const { cardId, fromColumn } = drag;
    drag = null;
    cancelAnimationFrame(raf);
    setDragId(null);
    setTarget(null);
    if (!t) return;
    const noop = t.column === fromColumn && t.index === columnIndexOf(props.cards, cardId);
    if (!noop) props.onCardsChange?.(moveCard(props.cards, cardId, t.column, t.index));
  };

  /* ---- render --------------------------------------------------------------- */

  const cardStyle = (card: KanbanCard): JSX.CSSProperties | undefined => {
    if (dragId() !== card.id || !drag) return undefined;
    const p = dragPos();
    return {
      left: `${p.left}px`,
      top: `${p.top}px`,
      width: `${drag.width}px`,
      height: `${drag.height}px`,
    };
  };

  return (
    <div
      ref={root}
      class={`fkanban ${props.class ?? ''}`}
      style={props.style}
      onPointerMove={rootMove}
      onPointerUp={rootUp}
      onPointerCancel={rootUp}
    >
      <For each={props.columns}>
        {(col) => (
          <section
            class="fkanban-col"
            data-col={col.id}
            classList={{
              'is-collapsed': col.collapsed,
              'is-drop-target': !!dragId() && target()?.column === col.id,
            }}
          >
            <header class="fkanban-col-head">
              <button
                type="button"
                class="fkanban-col-toggle"
                aria-label={col.collapsed ? `Expand ${col.title}` : `Collapse ${col.title}`}
                aria-expanded={!col.collapsed}
                onClick={() => props.onColumnToggle?.(col.id, !col.collapsed)}
              >
                <ChevronSvg />
              </button>
              <span class="fkanban-col-title">{col.title}</span>
              <Badge>{cardsInColumn(props.cards, col.id).length}</Badge>
              <Show when={!col.collapsed && props.onCardAdd}>
                <span class="fkanban-col-spacer" />
                <IconButton icon={PlusSvg} label={`Add card to ${col.title}`}
                            onClick={() => props.onCardAdd?.(col.id)} />
              </Show>
            </header>
            <Show when={!col.collapsed}>
              <div class="fkanban-body" data-col-body={col.id}>
                <For each={cardsInColumn(props.cards, col.id)}>
                  {(card, i) => (
                    <article
                      class="fkanban-card"
                      data-card={card.id}
                      classList={{ 'is-dragging': dragId() === card.id }}
                      style={{ order: i() * 2, ...cardStyle(card) }}
                      onPointerDown={cardDown(card)}
                    >
                      <Show when={card.title || props.onCardRemove}>
                        <div class="fkanban-card-head">
                          <span class="fkanban-card-title">{card.title}</span>
                          <Show when={props.onCardRemove}>
                            <IconButton icon={XSvg} label={`Remove ${card.title ?? card.id}`}
                                        onClick={() => props.onCardRemove?.(card.id)} />
                          </Show>
                        </div>
                      </Show>
                      <div class="fkanban-fields">
                        <CardFields card={card} fields={fieldsFor(card)} onCardChange={props.onCardChange} />
                      </div>
                      {props.children?.(card)}
                    </article>
                  )}
                </For>
                <Show when={dragId() && target()?.column === col.id}>
                  <div
                    class="fkanban-placeholder"
                    style={{ order: target()!.index * 2 - 1, height: `${drag?.height ?? 60}px` }}
                  />
                </Show>
              </div>
            </Show>
          </section>
        )}
      </For>
    </div>
  );
}
