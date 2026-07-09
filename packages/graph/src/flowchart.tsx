/* ---------------- Flowchart ------------------------------------------------- */
/* Display-oriented DAG: nodes without x/y are auto-laid-out left-to-right
   (longest-path layering + one barycenter pass). Edges reuse the .fgraph-edge
   state classes, so 'active' (marching ants) and 'broken' (flash) come free.
   Cycles tolerated: back-edges are ignored for layering but still drawn. */

import { Show, For, createMemo } from 'solid-js';
import type { JSX } from 'solid-js';
import { edgePath } from './ports';
import type { Point } from './ports';

export interface FlowNode {
  id: string;
  label: JSX.Element;
  /** Semantic tone applied as a `tone-*` class on the node box. */
  tone?: string;
}

export interface FlowEdge {
  from: string;
  to: string;
  label?: string;
  state?: 'active' | 'broken';
}

export interface FlowLayout {
  pos: Map<string, Point>;
  backEdges: Set<FlowEdge>;
  width: number;
  height: number;
}

export interface FlowchartProps {
  nodes?: FlowNode[];
  edges?: FlowEdge[];
  onNodeClick?: (id: string) => void;
  class?: string;
  style?: JSX.CSSProperties;
}

const FLOW_W = 160, FLOW_H = 36, GAP_X = 56, GAP_Y = 20;

export function layoutFlow(nodes: FlowNode[], edges: FlowEdge[]): FlowLayout {
  const ids = nodes.map((n) => n.id);
  const out = new Map<string, FlowEdge[]>(ids.map((id) => [id, []]));
  for (const e of edges) if (out.has(e.from)) out.get(e.from)!.push(e);

  /* DFS back-edge detection (gray-stack). */
  const state = new Map<string, number>();  // 0 unvisited, 1 in-stack, 2 done
  const backEdges = new Set<FlowEdge>();
  const visit = (id: string) => {
    state.set(id, 1);
    for (const e of out.get(id) ?? []) {
      const s = state.get(e.to) ?? 0;
      if (s === 1) backEdges.add(e);
      else if (s === 0 && out.has(e.to)) visit(e.to);
    }
    state.set(id, 2);
  };
  const indeg = new Map<string, number>(ids.map((id) => [id, 0]));
  for (const e of edges) if (!backEdges.has(e) && indeg.has(e.to)) indeg.set(e.to, indeg.get(e.to)! + 1);
  const roots = ids.filter((id) => edges.every((e) => e.to !== id));
  for (const r of roots.length ? roots : ids.slice(0, 1)) if (!state.get(r)) visit(r);
  for (const id of ids) if (!state.get(id)) visit(id);

  /* Longest-path layering over the back-edge-free DAG (relaxation passes). */
  const layer = new Map<string, number>(ids.map((id) => [id, 0]));
  for (let pass = 0; pass < ids.length; pass++) {
    let changed = false;
    for (const e of edges) {
      if (backEdges.has(e) || !layer.has(e.from) || !layer.has(e.to)) continue;
      if (layer.get(e.to)! < layer.get(e.from)! + 1) {
        layer.set(e.to, layer.get(e.from)! + 1);
        changed = true;
      }
    }
    if (!changed) break;
  }

  /* Group by layer (insertion order), one barycenter pass over predecessors. */
  const layers: string[][] = [];
  for (const n of nodes) {
    const l = layer.get(n.id)!;
    (layers[l] ??= []).push(n.id);
  }
  for (let l = 1; l < layers.length; l++) {
    const prevIdx = new Map((layers[l - 1] ?? []).map((id, i) => [id, i]));
    const bary = (id: string) => {
      const preds = edges.filter((e) => !backEdges.has(e) && e.to === id && prevIdx.has(e.from));
      if (!preds.length) return Number.MAX_SAFE_INTEGER;  // keep insertion order at the end
      return preds.reduce((s, e) => s + prevIdx.get(e.from)!, 0) / preds.length;
    };
    layers[l] = (layers[l] ?? []).map((id, i) => ({ id, b: bary(id), i }))
      .sort((a, b) => a.b - b.b || a.i - b.i).map((x) => x.id);
  }

  /* Coordinates with per-layer vertical centering. */
  const tallest = Math.max(...layers.map((l) => l?.length ?? 0));
  const totalH = tallest * FLOW_H + (tallest - 1) * GAP_Y;
  const pos = new Map<string, Point>();
  layers.forEach((col, l) => {
    const colH = col.length * FLOW_H + (col.length - 1) * GAP_Y;
    const y0 = (totalH - colH) / 2;
    col.forEach((id, i) => pos.set(id, {
      x: 16 + l * (FLOW_W + GAP_X),
      y: 16 + y0 + i * (FLOW_H + GAP_Y),
    }));
  });
  return {
    pos, backEdges,
    width: 32 + layers.length * FLOW_W + (layers.length - 1) * GAP_X,
    height: 32 + totalH,
  };
}

export function Flowchart(props: FlowchartProps) {
  const layout = createMemo(() => layoutFlow(props.nodes ?? [], props.edges ?? []));
  const anchor = (id: string, side: 'in' | 'out'): Point | null => {
    const p = layout().pos.get(id);
    if (!p) return null;
    return { x: side === 'out' ? p.x + FLOW_W : p.x, y: p.y + FLOW_H / 2 };
  };
  return (
    <div class="fchart-scroll">
      <div class={`fflow ${props.class ?? ''}`}
           style={{ width: `${layout().width}px`, height: `${layout().height}px`, ...props.style }}>
        <svg class="fgraph-svg">
          <For each={props.edges}>
            {(e) => {
              const a = createMemo(() => anchor(e.from, 'out'));
              const b = createMemo(() => anchor(e.to, 'in'));
              return (
                <Show when={a() && b()}>
                  <path class="fgraph-edge" d={edgePath(a()!, b()!)}
                        classList={{ 'is-active': e.state === 'active', 'is-broken': e.state === 'broken' }} />
                  <Show when={e.label}>
                    {(() => {
                      const lx = () => (a()!.x + b()!.x) / 2;
                      const ly = () => (a()!.y + b()!.y) / 2;
                      return (
                        <>
                          <rect class="fflow-edge-label-bg" x={lx() - e.label!.length * 3.2 - 4} y={ly() - 8}
                                width={e.label!.length * 6.4 + 8} height="16" rx="3" />
                          <text class="fflow-edge-label" x={lx()} y={ly()} text-anchor="middle" dominant-baseline="central">
                            {e.label}
                          </text>
                        </>
                      );
                    })()}
                  </Show>
                </Show>
              );
            }}
          </For>
        </svg>
        <For each={props.nodes}>
          {(n) => {
            const p = () => layout().pos.get(n.id);
            return (
              <Show when={p()}>
                <div class={`fflow-node${n.tone ? ` tone-${n.tone}` : ''}`}
                     classList={{ 'is-clickable': !!props.onNodeClick }}
                     style={{ left: `${p()!.x}px`, top: `${p()!.y}px`, width: `${FLOW_W}px` }}
                     onClick={() => props.onNodeClick?.(n.id)}>
                  {n.label}
                </div>
              </Show>
            );
          }}
        </For>
      </div>
    </div>
  );
}
