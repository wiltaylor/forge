/* Forge node graph — needs @forge/graph/styles.css (the "Node graph" section
   extracted from console.css). Controlled component: nodes/edges/selection come
   from props; interactions are reported through callbacks and the consumer
   updates its own store.

   <NodeGraph
     nodes={[{ id, x, y, title, w?, inputs: [{id, type, label?}], outputs: [...] }]}
     edges={[{ id, from: {node, port}, to: {node, port}, state?: 'active'|'broken' }]}
     selected={null | { kind: 'node'|'edge', id }}
     onNodeMove={(id, x, y) => ...}   // fires per pointermove during a drag
     onConnect={({ from, to }) => ...} // consumer appends the edge
     onSelect={(sel|null) => ...}      // null = background click
     onDelete={(sel) => ...}           // Delete/Backspace with a selection
   >
     {(node) => <optional body JSX per node>}
   </NodeGraph>

   Imports only solid-js — no coupling to @forge/ui. */

import { Show, For, createSignal, createMemo } from 'solid-js';
import type { JSX } from 'solid-js';
import { portColor, edgePath } from './ports';
import type { Point } from './ports';

/** A single input or output port on a node. */
export interface GraphPort {
  id: string;
  type?: string;
  label?: JSX.Element;
}

export interface GraphNode {
  id: string;
  x: number;
  y: number;
  title: JSX.Element;
  /** Pixel width override (default 180). */
  w?: number;
  inputs?: GraphPort[];
  outputs?: GraphPort[];
}

/** One end of an edge: node id + port id. */
export interface GraphEdgeEnd {
  node: string;
  port: string;
}

export interface GraphEdge {
  id: string;
  from: GraphEdgeEnd;
  to: GraphEdgeEnd;
  state?: 'active' | 'broken';
}

export interface GraphSelection {
  kind: 'node' | 'edge';
  id: string;
}

export interface NodeGraphProps {
  nodes?: GraphNode[];
  edges?: GraphEdge[];
  selected?: GraphSelection | null;
  /** Fires per pointermove during a drag. */
  onNodeMove?: (id: string, x: number, y: number) => void;
  /** Consumer appends the edge. */
  onConnect?: (conn: { from: GraphEdgeEnd; to: GraphEdgeEnd }) => void;
  /** null = background click. */
  onSelect?: (sel: GraphSelection | null) => void;
  /** Delete/Backspace with a selection. */
  onDelete?: (sel: GraphSelection) => void;
  class?: string;
  style?: JSX.CSSProperties | string;
  /** Optional body JSX per node. */
  children?: (node: GraphNode) => JSX.Element;
}

const compatible = (a: string | undefined, b: string | undefined) => a === b || a === 'any' || b === 'any';

/* Geometry constants — port dot positions and edge anchors share these, so
   they coincide by construction (no DOM measurement during drag). */
const NODE_W = 180;
const HEAD_H = 33;
const ROW_H = 24;

/* Input rows render first, then output rows — the output anchor offsets by
   the input count so dots and edge endpoints coincide. */
const inAnchor = (n: GraphNode, i: number): Point => ({ x: n.x, y: n.y + HEAD_H + ROW_H * i + ROW_H / 2 });
const outAnchor = (n: GraphNode, i: number): Point => ({
  x: n.x + (n.w ?? NODE_W),
  y: n.y + HEAD_H + ROW_H * ((n.inputs?.length ?? 0) + i) + ROW_H / 2,
});

interface PendingConn {
  from: GraphEdgeEnd;
  type?: string;
  x: number;
  y: number;
}

/* ---------------- NodeGraph ------------------------------------------------- */
export function NodeGraph(props: NodeGraphProps) {
  let root!: HTMLDivElement;
  let drag: { id: string; dx: number; dy: number } | null = null;  // plain variable, not reactive (per-frame)
  const [pending, setPending] = createSignal<PendingConn | null>(null);  // {from, type, x, y}

  const nodeById = createMemo(() => {
    const m: Record<string, GraphNode> = {};
    for (const n of props.nodes ?? []) m[n.id] = n;
    return m;
  });

  const toCanvas = (e: PointerEvent) => {
    const r = root.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  };

  const headDown = (node: GraphNode) => (e: PointerEvent) => {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const p = toCanvas(e);
    drag = { id: node.id, dx: p.x - node.x, dy: p.y - node.y };
    props.onSelect?.({ kind: 'node', id: node.id });
  };
  const rootMove = (e: PointerEvent) => {
    if (drag) {
      const p = toCanvas(e);
      props.onNodeMove?.(drag.id, p.x - drag.dx, p.y - drag.dy);
    } else if (pending()) {
      const p = toCanvas(e);
      setPending((prev) => ({ ...prev!, x: p.x, y: p.y }));
    }
  };
  const rootUp = () => {
    drag = null;
    setPending(null);
  };

  const portDown = (node: GraphNode, port: GraphPort) => (e: PointerEvent) => {
    e.stopPropagation();
    e.preventDefault();
    const p = toCanvas(e);
    setPending({ from: { node: node.id, port: port.id }, type: port.type, x: p.x, y: p.y });
  };
  const portUp = (node: GraphNode, port: GraphPort) => (e: PointerEvent) => {
    const p = pending();
    if (p && compatible(p.type, port.type) && p.from.node !== node.id) {
      e.stopPropagation();
      props.onConnect?.({ from: p.from, to: { node: node.id, port: port.id } });
    }
    setPending(null);
  };

  const edgeGeom = (edge: GraphEdge) => {
    const from = nodeById()[edge.from.node];
    const to = nodeById()[edge.to.node];
    if (!from || !to) return null;
    const oi = (from.outputs ?? []).findIndex((pt) => pt.id === edge.from.port);
    const ii = (to.inputs ?? []).findIndex((pt) => pt.id === edge.to.port);
    if (oi < 0 || ii < 0) return null;
    return {
      d: edgePath(outAnchor(from, oi), inAnchor(to, ii)),
      color: portColor((from.outputs ?? [])[oi]?.type),
    };
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if ((e.key === 'Delete' || e.key === 'Backspace') && props.selected &&
        !/^(INPUT|TEXTAREA|SELECT)$/.test((e.target as Element).tagName)) {
      e.preventDefault();
      props.onDelete?.(props.selected);
    }
  };

  const isSel = (kind: GraphSelection['kind'], id: string) => props.selected?.kind === kind && props.selected?.id === id;
  const pendingStart = () => {
    const p = pending();
    if (!p) return null;
    const n = nodeById()[p.from.node];
    if (!n) return null;
    const oi = (n.outputs ?? []).findIndex((pt) => pt.id === p.from.port);
    return oi < 0 ? null : outAnchor(n, oi);
  };

  return (
    <div class={`fgraph ${props.class ?? ''}`} style={props.style} ref={root} tabindex="0"
         onPointerMove={rootMove} onPointerUp={rootUp} onKeyDown={onKeyDown}
         onPointerDown={(e) => { if (e.target === root) props.onSelect?.(null); }}>
      <svg class="fgraph-svg">
        <For each={props.edges}>
          {(edge) => {
            const geom = createMemo(() => edgeGeom(edge));
            return (
              <Show when={geom()}>
                <g>
                  <path class="fgraph-edge-hit" d={geom()!.d}
                        onClick={() => props.onSelect?.({ kind: 'edge', id: edge.id })} />
                  <path class="fgraph-edge" d={geom()!.d}
                        style={{ stroke: edge.state || isSel('edge', edge.id) ? undefined : geom()!.color }}
                        classList={{
                          'is-active': edge.state === 'active',
                          'is-broken': edge.state === 'broken',
                          'is-selected': isSel('edge', edge.id),
                        }} />
                </g>
              </Show>
            );
          }}
        </For>
        <Show when={pending() && pendingStart()}>
          <path class="fgraph-pending" d={edgePath(pendingStart()!, { x: pending()!.x, y: pending()!.y })} />
        </Show>
      </svg>
      <For each={props.nodes}>
        {(node) => (
          <div class="fgraph-node" classList={{ 'is-selected': isSel('node', node.id) }}
               style={{ left: `${node.x}px`, top: `${node.y}px`, width: node.w ? `${node.w}px` : undefined }}
               onPointerDown={(e) => e.stopPropagation()}>
            <div class="fgraph-node-head" onPointerDown={headDown(node)}>
              <span class="fgraph-node-title">{node.title}</span>
            </div>
            <For each={node.inputs ?? []}>
              {(port) => (
                <div class="fgraph-row">
                  <span class="fgraph-port fgraph-port-in" style={{ background: portColor(port.type) }}
                        onPointerDown={(e) => e.stopPropagation()} onPointerUp={portUp(node, port)} />
                  {port.label ?? port.id}
                </div>
              )}
            </For>
            <For each={node.outputs ?? []}>
              {(port) => (
                <div class="fgraph-row fgraph-row-out">
                  {port.label ?? port.id}
                  <span class="fgraph-port fgraph-port-out" style={{ background: portColor(port.type) }}
                        onPointerDown={portDown(node, port)} />
                </div>
              )}
            </For>
            <Show when={props.children}>
              <div class="fgraph-node-body">{props.children!(node)}</div>
            </Show>
          </div>
        )}
      </For>
    </div>
  );
}
