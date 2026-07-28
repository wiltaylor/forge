/* The block document schema — the JSON interchange contract shared verbatim
   with crates/forge-blocks (Rust). Any shape change must land on both sides
   (see crates/forge-blocks/tests/schema.rs for the literal fixtures). */
import type { JSX } from 'solid-js';
import type { IconComponent } from '@forge/ui';

export const DOC_VERSION = 1;

export interface BlockDocument {
  version: typeof DOC_VERSION;
  blocks: Block[];
}

export type ListStyle = 'bullet' | 'number' | 'todo';
export type AdmonitionTone = 'info' | 'success' | 'warning' | 'danger';

export interface BlockColumn {
  ratio: number;
  blocks: Block[];
}

export interface ChartSeries {
  name: string;
  values: number[];
}

/** A labelled point annotation on a line chart; `category` indexes into the
    chart's `categories`. */
export interface ChartPoint {
  label: string;
  category: number;
  value: number;
}

export interface PieSlice {
  label: string;
  value: number;
}

export type DiagramDirection = 'right' | 'down';
export type DiagramNodeKind = 'process' | 'decision' | 'terminator' | 'node';
export type DiagramEdgeKind = 'solid' | 'dashed';

export interface DiagramNode {
  id: string;
  kind: DiagramNodeKind;
  text: string;
}

export interface DiagramEdge {
  from: string;
  to: string;
  label?: string;
  kind?: DiagramEdgeKind;
}

export type ParticipantKind = 'box' | 'actor' | 'external';
export type MessageKind = 'sync' | 'async' | 'reply';

export interface SeqParticipant {
  id: string;
  name?: string;
  kind?: ParticipantKind;
}

export interface SeqMessage {
  from: string;
  to: string;
  text?: string;
  kind?: MessageKind;
}

/** A note anchored under message index `at`. */
export interface SeqNote {
  at: number;
  text: string;
}

export interface StateNode {
  id: string;
  name?: string;
  initial?: boolean;
  final?: boolean;
}

export interface StateTransition {
  from: string;
  to: string;
  trigger?: string;
  guard?: string;
}

/** `key` is the row's stable identifier (wdoc uses it as an edge target). */
export interface NodeTableRow {
  key?: string;
  md: string;
}

export interface TreeNode {
  title: string;
  icon?: string;
  children?: TreeNode[];
}

export type TimelineDirection = 'horizontal' | 'vertical';
export type TimelineSide = 'near' | 'far';

/** `from`/`to`/`on` are ISO-8601 date strings. */
export interface TimelinePhase {
  label: string;
  from: string;
  to: string;
}

export interface TimelineItem {
  label: string;
  on: string;
  side?: TimelineSide;
}

export type Block = { id: string } & (
  | { type: 'paragraph'; md: string }
  | { type: 'heading'; level: 1 | 2 | 3 | 4; md: string }
  | { type: 'list_item'; style: ListStyle; checked?: boolean; indent: number; md: string }
  | { type: 'quote'; md: string }
  | { type: 'divider' }
  | { type: 'code'; lang: string; code: string }
  /** Cells are inline-markdown strings. */
  | { type: 'table'; header: string[]; rows: string[][] }
  | { type: 'admonition'; tone: AdmonitionTone; title: string; md: string }
  /** One level only — column cells never contain another `columns` block. */
  | { type: 'columns'; columns: BlockColumn[] }
  /** Consumer-defined block; `kind` selects the BlockDef supplied via props. */
  | { type: 'custom'; kind: string; data: unknown }
  | { type: 'image'; src: string; alt: string; width?: number; height?: number }
  /** `src` is a local/remote file path or a YouTube/Vimeo URL. */
  | { type: 'video'; src: string; poster?: string; title?: string; width?: number; height?: number }
  /** LaTeX source; renderers typeset it if they can, else show the source. */
  | { type: 'math'; tex: string }
  | {
      type: 'bar_chart';
      title?: string;
      x_label?: string;
      y_label?: string;
      categories: string[];
      series: ChartSeries[];
      y_min?: number;
      y_max?: number;
    }
  | {
      type: 'line_chart';
      title?: string;
      x_label?: string;
      y_label?: string;
      categories: string[];
      series: ChartSeries[];
      y_min?: number;
      y_max?: number;
      points?: ChartPoint[];
      point_labels?: boolean;
    }
  | { type: 'pie_chart'; title?: string; slices: PieSlice[] }
  /** Auto-laid-out flowchart graph. */
  | { type: 'diagram'; direction?: DiagramDirection; nodes: DiagramNode[]; edges: DiagramEdge[] }
  | {
      type: 'sequence_diagram';
      participants: SeqParticipant[];
      messages: SeqMessage[];
      notes?: SeqNote[];
    }
  | { type: 'state_diagram'; states: StateNode[]; transitions: StateTransition[] }
  /** DB/class-diagram style row table; an empty `title` means headerless. */
  | { type: 'node_table'; title: string; rows: NodeTableRow[] }
  | { type: 'tree'; nodes: TreeNode[] }
  | {
      type: 'timeline';
      title?: string;
      direction?: TimelineDirection;
      phases?: TimelinePhase[];
      items: TimelineItem[];
    }
  | {
      type: 'chapter_header';
      title: string;
      kicker?: string;
      reading_time?: string;
      updated?: string;
      version?: string;
    }
  /** Footnote definition; inline `[^label]` references link to it.
      (`label`, not `id` — the block's `id` sits beside these fields.) */
  | { type: 'footnote'; label: string; md: string }
);

export type BlockType = Block['type'];
export type TextBlock = Extract<Block, { md: string }>;

/** A block without its id — distributed over the union (plain `Omit`
    collapses union members to their common properties). */
export type BlockData = Block extends infer B ? (B extends Block ? Omit<B, 'id'> : never) : never;

/** Blocks whose `md` body is edited with the shared text keyboard model. */
export function isTextBlock(b: Block): b is TextBlock {
  return 'md' in b;
}

/** Data-block types: rendered from structured fields and edited as raw JSON
    source rather than through a bespoke editor. Mirrors Rust `is_data()`
    (footnote is text-bearing, math edits its `tex` as plain text on web). */
export const DATA_TYPES = [
  'image',
  'video',
  'math',
  'bar_chart',
  'line_chart',
  'pie_chart',
  'diagram',
  'sequence_diagram',
  'state_diagram',
  'node_table',
  'tree',
  'timeline',
  'chapter_header',
] as const satisfies readonly BlockType[];

export type DataBlock = Extract<Block, { type: (typeof DATA_TYPES)[number] }>;

export function isDataBlock(b: Block): b is DataBlock {
  return (DATA_TYPES as readonly string[]).includes(b.type);
}

/** Consumer-defined custom block: how it inserts, renders, and edits.
    `render` output is consumer code — it bypasses the parser's XSS safety,
    so treat `data` as untrusted when rendering user-provided documents. */
export interface BlockDef {
  label: string;
  icon?: IconComponent;
  /** Initial `data` for a freshly inserted block. */
  create: () => unknown;
  render: (props: { data: unknown }) => JSX.Element;
  /** Focused UI; omitted = render + block menu only. */
  edit?: (props: { data: unknown; onChange: (data: unknown) => void }) => JSX.Element;
}

let counter = 0;

/** A fresh block id (web side uses UUIDs; any unique string is valid). */
export function newId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `blk_${Date.now().toString(36)}_${(counter++).toString(36)}`;
}

/** A fresh empty block of the given type with sensible defaults. */
export function createBlock(type: BlockType): Block {
  const id = newId();
  switch (type) {
    case 'paragraph':
      return { id, type, md: '' };
    case 'heading':
      return { id, type, level: 1, md: '' };
    case 'list_item':
      return { id, type, style: 'bullet', indent: 0, md: '' };
    case 'quote':
      return { id, type, md: '' };
    case 'divider':
      return { id, type };
    case 'code':
      return { id, type, lang: '', code: '' };
    case 'table':
      return { id, type, header: ['', '', ''], rows: [['', '', ''], ['', '', '']] };
    case 'admonition':
      return { id, type, tone: 'info', title: '', md: '' };
    case 'columns':
      return {
        id,
        type,
        columns: [
          { ratio: 0.5, blocks: [createBlock('paragraph')] },
          { ratio: 0.5, blocks: [createBlock('paragraph')] },
        ],
      };
    case 'custom':
      return { id, type, kind: '', data: null };
    /* Data-block starters mirror Rust `starter_kind` so every kit inserts
       the same shapes. */
    case 'image':
      return { id, type, src: '', alt: '' };
    case 'video':
      return { id, type, src: '' };
    case 'math':
      return { id, type, tex: '' };
    case 'bar_chart':
      return {
        id,
        type,
        categories: ['A', 'B', 'C'],
        series: [{ name: 'Series 1', values: [3, 5, 4] }],
      };
    case 'line_chart':
      return {
        id,
        type,
        categories: ['A', 'B', 'C'],
        series: [{ name: 'Series 1', values: [3, 5, 4] }],
      };
    case 'pie_chart':
      return {
        id,
        type,
        slices: [
          { label: 'A', value: 3 },
          { label: 'B', value: 5 },
        ],
      };
    case 'diagram':
      return {
        id,
        type,
        nodes: [
          { id: 'start', kind: 'terminator', text: 'Start' },
          { id: 'work', kind: 'process', text: 'Work' },
          { id: 'done', kind: 'terminator', text: 'Done' },
        ],
        edges: [
          { from: 'start', to: 'work' },
          { from: 'work', to: 'done' },
        ],
      };
    case 'sequence_diagram':
      return {
        id,
        type,
        participants: [
          { id: 'a', name: 'Client' },
          { id: 'b', name: 'Server' },
        ],
        messages: [
          { from: 'a', to: 'b', text: 'request' },
          { from: 'b', to: 'a', text: 'response', kind: 'reply' },
        ],
      };
    case 'state_diagram':
      return {
        id,
        type,
        states: [
          { id: 'idle', name: 'Idle', initial: true },
          { id: 'done', name: 'Done', final: true },
        ],
        transitions: [{ from: 'idle', to: 'done', trigger: 'finish' }],
      };
    case 'node_table':
      return { id, type, title: 'Table', rows: [{ md: 'row' }] };
    case 'tree':
      return { id, type, nodes: [{ title: 'root', children: [{ title: 'child' }] }] };
    case 'timeline':
      return { id, type, items: [{ label: 'Start', on: '2026-01-01' }] };
    case 'chapter_header':
      return { id, type, title: 'Title' };
    case 'footnote':
      return { id, type, label: 'note-1', md: '' };
  }
}

/** An empty document holding a single empty paragraph (the editor invariant:
    a document is never blockless). */
export function emptyDocument(): BlockDocument {
  return { version: DOC_VERSION, blocks: [createBlock('paragraph')] };
}
