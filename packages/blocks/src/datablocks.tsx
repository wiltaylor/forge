/* Read-only renderers for the data block kinds (image, video, math, charts,
   diagrams, node table, tree, timeline, chapter header, footnote). Charts and
   flowcharts reuse @forge/charts and @forge/graph; the rest are plain
   HTML/SVG following the charts.tsx idiom. */
import { For, Index, Show, createMemo, createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { BarChart, LineChart, PieChart } from '@forge/charts';
import { Flowchart } from '@forge/graph';
import type { FlowEdge, FlowNode } from '@forge/graph';
import { InlineMd } from './inline';
import type { Block, TreeNode } from './types';

type Kind<T extends Block['type']> = Extract<Block, { type: T }>;

/** The slice of RenderCtx the data renderers need (avoids an import cycle
    with render.tsx). */
export interface DataCtx {
  emoji?: Record<string, string>;
  linkTarget?: '_blank' | '_self';
  renderMath?: (tex: string) => JSX.Element;
}

const inline = (ctx: DataCtx, md: string) => (
  <InlineMd md={md} emoji={ctx.emoji} linkTarget={ctx.linkTarget} />
);

/* ---------------- media ---------------- */

export function ImageView(props: { block: Kind<'image'> }): JSX.Element {
  return (
    <figure class="fbk-img">
      <img
        src={props.block.src}
        alt={props.block.alt}
        width={props.block.width}
        height={props.block.height}
        loading="lazy"
      />
      <Show when={props.block.alt}>
        <figcaption>{props.block.alt}</figcaption>
      </Show>
    </figure>
  );
}

/** YouTube/Vimeo URL → embeddable iframe src; null = plain file. */
export function videoEmbed(src: string): string | null {
  const yt = /(?:youtube\.com\/watch\?v=|youtu\.be\/)([\w-]{6,})/.exec(src);
  if (yt) return `https://www.youtube-nocookie.com/embed/${yt[1]}`;
  const vimeo = /vimeo\.com\/(\d+)/.exec(src);
  if (vimeo) return `https://player.vimeo.com/video/${vimeo[1]}`;
  return null;
}

export function VideoView(props: { block: Kind<'video'> }): JSX.Element {
  const [playing, setPlaying] = createSignal(false);
  const embed = () => videoEmbed(props.block.src);
  const size = () => ({
    'aspect-ratio': props.block.width && props.block.height
      ? `${props.block.width} / ${props.block.height}`
      : '16 / 9',
  });
  return (
    <div class="fbk-video" style={size()}>
      <Show
        when={playing()}
        fallback={
          <button
            type="button"
            class="fbk-video-facade"
            style={{ 'background-image': props.block.poster ? `url(${props.block.poster})` : undefined }}
            onClick={() => setPlaying(true)}
          >
            <span class="fbk-video-play">▶</span>
            <Show when={props.block.title}>
              <span class="fbk-video-title">{props.block.title}</span>
            </Show>
          </button>
        }
      >
        <Show
          when={embed()}
          fallback={<video src={props.block.src} poster={props.block.poster} controls autoplay />}
        >
          {(url) => (
            <iframe
              src={`${url()}?autoplay=1`}
              title={props.block.title ?? 'video'}
              allow="autoplay; fullscreen; picture-in-picture"
              allowfullscreen
            />
          )}
        </Show>
      </Show>
    </div>
  );
}

export function MathView(props: { block: Kind<'math'>; ctx: DataCtx }): JSX.Element {
  return (
    <div class="fbk-math">
      <Show when={props.ctx.renderMath} fallback={<code>{props.block.tex}</code>}>
        {(render) => render()(props.block.tex)}
      </Show>
    </div>
  );
}

/* ---------------- charts ---------------- */

function ChartFrame(props: { title?: string; xLabel?: string; yLabel?: string; children: JSX.Element }) {
  return (
    <figure class="fbk-chart">
      <Show when={props.title}>
        <figcaption class="fbk-chart-title">{props.title}</figcaption>
      </Show>
      {props.children}
      <Show when={props.xLabel || props.yLabel}>
        <div class="fbk-chart-axes">
          <Show when={props.yLabel}>
            <span>↑ {props.yLabel}</span>
          </Show>
          <Show when={props.xLabel}>
            <span>→ {props.xLabel}</span>
          </Show>
        </div>
      </Show>
    </figure>
  );
}

export function BarChartView(props: { block: Kind<'bar_chart'> }): JSX.Element {
  const series = () =>
    props.block.series.map((s) => ({
      label: s.name,
      data: props.block.categories.map((c, i) => ({ label: c, value: s.values[i] ?? 0 })),
    }));
  return (
    <ChartFrame title={props.block.title} xLabel={props.block.x_label} yLabel={props.block.y_label}>
      <BarChart series={series()} height={200} />
    </ChartFrame>
  );
}

export function LineChartView(props: { block: Kind<'line_chart'> }): JSX.Element {
  const series = () =>
    props.block.series.map((s) => ({
      label: s.name,
      points: s.values.map((y, x) => ({ x, y })),
    }));
  return (
    <ChartFrame title={props.block.title} xLabel={props.block.x_label} yLabel={props.block.y_label}>
      <LineChart series={series()} xLabels={props.block.categories} height={200} />
      <Show when={props.block.points?.length}>
        <div class="fbk-chart-points">
          <For each={props.block.points}>
            {(p) => (
              <span>
                {p.label}: {props.block.categories[p.category] ?? p.category} → {p.value}
              </span>
            )}
          </For>
        </div>
      </Show>
    </ChartFrame>
  );
}

export function PieChartView(props: { block: Kind<'pie_chart'> }): JSX.Element {
  return (
    <ChartFrame title={props.block.title}>
      <PieChart data={props.block.slices.map((s) => ({ label: s.label, value: s.value }))} />
    </ChartFrame>
  );
}

/* ---------------- diagrams ---------------- */

export function DiagramView(props: { block: Kind<'diagram'> }): JSX.Element {
  const nodes = (): FlowNode[] =>
    props.block.nodes.map((n) => ({
      id: n.id,
      label: n.text,
      tone: n.kind === 'process' ? undefined : n.kind,
    }));
  const edges = (): FlowEdge[] =>
    props.block.edges.map((e) => ({ from: e.from, to: e.to, label: e.label }));
  return (
    <div class="fbk-diagram">
      <Flowchart nodes={nodes()} edges={edges()} />
    </div>
  );
}

export function StateDiagramView(props: { block: Kind<'state_diagram'> }): JSX.Element {
  const nodes = (): FlowNode[] =>
    props.block.states.map((s) => ({
      id: s.id,
      label: `${s.initial ? '● ' : ''}${s.final ? '◉ ' : ''}${s.name ?? s.id}`,
      tone: s.initial ? 'initial' : s.final ? 'final' : undefined,
    }));
  const edges = (): FlowEdge[] =>
    props.block.transitions.map((t) => ({
      from: t.from,
      to: t.to,
      label: t.trigger ? `${t.trigger}${t.guard ? ` [${t.guard}]` : ''}` : undefined,
    }));
  return (
    <div class="fbk-diagram">
      <Flowchart nodes={nodes()} edges={edges()} />
    </div>
  );
}

const SEQ = { colW: 170, headH: 30, rowH: 30, pad: 8 };

export function SequenceDiagramView(props: { block: Kind<'sequence_diagram'> }): JSX.Element {
  const cols = createMemo(() => new Map(props.block.participants.map((p, i) => [p.id, i])));
  const x = (id: string) => (cols().get(id) ?? 0) * SEQ.colW + SEQ.colW / 2;
  const noteRows = createMemo(() => {
    const map = new Map<number, string[]>();
    for (const n of props.block.notes ?? []) {
      const at = Math.min(n.at, Math.max(0, props.block.messages.length - 1));
      map.set(at, [...(map.get(at) ?? []), n.text]);
    }
    return map;
  });
  /* Rows: one per message plus one per note line under its anchor. */
  const rows = createMemo(() => {
    const out: ({ kind: 'msg'; i: number } | { kind: 'note'; text: string })[] = [];
    props.block.messages.forEach((_, i) => {
      out.push({ kind: 'msg', i });
      for (const text of noteRows().get(i) ?? []) out.push({ kind: 'note', text });
    });
    return out;
  });
  const width = () => Math.max(1, props.block.participants.length) * SEQ.colW;
  const height = () => SEQ.headH + SEQ.pad + rows().length * SEQ.rowH + SEQ.pad;
  const rowY = (r: number) => SEQ.headH + SEQ.pad + r * SEQ.rowH + SEQ.rowH / 2;

  return (
    <div class="fbk-seq">
      <svg width={width()} height={height()} role="img" aria-label="Sequence diagram">
        <For each={props.block.participants}>
          {(p) => (
            <>
              <line
                class="fbk-seq-lifeline"
                x1={x(p.id)}
                x2={x(p.id)}
                y1={SEQ.headH}
                y2={height()}
              />
              <rect
                class="fbk-seq-head"
                classList={{ 'is-actor': p.kind === 'actor', 'is-external': p.kind === 'external' }}
                x={x(p.id) - SEQ.colW / 2 + 10}
                y={2}
                width={SEQ.colW - 20}
                height={SEQ.headH - 6}
                rx={4}
              />
              <text class="fbk-seq-name" x={x(p.id)} y={SEQ.headH / 2 + 1} text-anchor="middle" dominant-baseline="central">
                {p.name ?? p.id}
              </text>
            </>
          )}
        </For>
        <Index each={rows()}>
          {(row, r) => {
            const item = row();
            if (item.kind === 'note') {
              return (
                <text class="fbk-seq-note" x={SEQ.pad} y={rowY(r)} dominant-baseline="central">
                  ▹ {item.text}
                </text>
              );
            }
            const m = props.block.messages[item.i]!;
            const x1 = x(m.from);
            const x2 = x(m.to);
            const y = rowY(r);
            const dir = x2 >= x1 ? 1 : -1;
            const tip = x2 - dir * 6;
            return (
              <>
                <line
                  class="fbk-seq-msg"
                  classList={{ 'is-dashed': m.kind === 'async' || m.kind === 'reply' }}
                  x1={x1}
                  x2={x2 - dir * 4}
                  y1={y}
                  y2={y}
                />
                <polygon
                  class="fbk-seq-arrow"
                  points={`${x2},${y} ${tip},${y - 4} ${tip},${y + 4}`}
                />
                <Show when={m.text}>
                  <text
                    class="fbk-seq-label"
                    x={(x1 + x2) / 2}
                    y={y - 6}
                    text-anchor="middle"
                  >
                    {m.text}
                  </text>
                </Show>
              </>
            );
          }}
        </Index>
      </svg>
    </div>
  );
}

/* ---------------- structure ---------------- */

export function NodeTableView(props: { block: Kind<'node_table'>; ctx: DataCtx }): JSX.Element {
  return (
    <div class="fbk-ntable">
      <Show when={props.block.title}>
        <div class="fbk-ntable-title">{props.block.title}</div>
      </Show>
      <For each={props.block.rows}>
        {(row) => (
          <div class="fbk-ntable-row">
            <span class="fbk-ntable-port" />
            <span>{inline(props.ctx, row.md)}</span>
          </div>
        )}
      </For>
    </div>
  );
}

export function TreeView(props: { block: Kind<'tree'> }): JSX.Element {
  return (
    <div class="fbk-tree">
      <TreeRows nodes={props.block.nodes} prefix="" />
    </div>
  );
}

function TreeRows(props: { nodes: TreeNode[]; prefix: string }) {
  return (
    <For each={props.nodes}>
      {(node, i) => {
        const last = () => i() === props.nodes.length - 1;
        return (
          <>
            <div class="fbk-tree-row">
              <span class="fbk-tree-guides">{props.prefix + (last() ? '└─ ' : '├─ ')}</span>
              <Show when={node.icon}>
                <span class="fbk-tree-icon">{node.icon} </span>
              </Show>
              <span>{node.title}</span>
            </div>
            <Show when={node.children?.length}>
              <TreeRows nodes={node.children!} prefix={props.prefix + (last() ? '   ' : '│  ')} />
            </Show>
          </>
        );
      }}
    </For>
  );
}

export function TimelineView(props: { block: Kind<'timeline'> }): JSX.Element {
  const items = createMemo(() =>
    [...props.block.items].sort((a, b) => Date.parse(a.on) - Date.parse(b.on)),
  );
  const phaseFor = (on: string) =>
    props.block.phases?.find((p) => Date.parse(on) >= Date.parse(p.from) && Date.parse(on) < Date.parse(p.to));
  return (
    <div class="fbk-timeline" classList={{ 'is-horizontal': props.block.direction === 'horizontal' }}>
      <Show when={props.block.title}>
        <div class="fbk-timeline-title">{props.block.title}</div>
      </Show>
      <Show when={props.block.phases?.length}>
        <div class="fbk-timeline-phases">
          <For each={props.block.phases}>
            {(p) => (
              <span class="fbk-timeline-phase">
                {p.label} <span class="fbk-timeline-range">{p.from} → {p.to}</span>
              </span>
            )}
          </For>
        </div>
      </Show>
      <div class="fbk-timeline-items">
        <For each={items()}>
          {(item) => (
            <div class="fbk-timeline-item">
              <span class="fbk-timeline-dot" />
              <span class="fbk-timeline-label">{item.label}</span>
              <span class="fbk-timeline-date">
                {item.on}
                <Show when={phaseFor(item.on)}>{(p) => <> · {p().label}</>}</Show>
              </span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}

export function ChapterHeaderView(props: { block: Kind<'chapter_header'>; ctx: DataCtx }): JSX.Element {
  const meta = () =>
    [props.block.reading_time, props.block.updated, props.block.version].filter(Boolean);
  return (
    <header class="fbk-chapter">
      <Show when={props.block.kicker}>
        <p class="fbk-chapter-kicker">{props.block.kicker}</p>
      </Show>
      <h1 class="fbk-chapter-title">{inline(props.ctx, props.block.title)}</h1>
      <Show when={meta().length}>
        <p class="fbk-chapter-meta">
          <For each={meta()}>{(m, i) => <>{i() > 0 && ' · '}{m}</>}</For>
        </p>
      </Show>
    </header>
  );
}

export function FootnoteView(props: { block: Kind<'footnote'>; ctx: DataCtx }): JSX.Element {
  return (
    <div class="fbk-footnote" id={`fn-${props.block.label}`}>
      <sup class="fbk-footnote-label">[{props.block.label}]</sup>{' '}
      {inline(props.ctx, props.block.md)}
    </div>
  );
}
