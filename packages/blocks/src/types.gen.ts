/* GENERATED FILE — do not edit by hand.
   Source:     crates/forge-blocks/src/registry.rs
   Read from:  contract/blocks-registry.json   (`just generate-blocks` rewrites it)
   Regenerate: just generate   (`just check` fails while this file is stale) */
import { newId } from './id';
import type {
  AdmonitionTone,
  BlockColumn,
  ChartPoint,
  ChartSeries,
  DiagramDirection,
  DiagramEdge,
  DiagramNode,
  ListStyle,
  NodeTableRow,
  PieSlice,
  SeqMessage,
  SeqNote,
  SeqParticipant,
  StateNode,
  StateTransition,
  TimelineDirection,
  TimelineItem,
  TimelinePhase,
  TreeNode,
} from './wire';

/** Every block the schema defines: the block's own `id`, then the `type` tag
    and the fields beside it that make up the wire shape `crates/forge-blocks`
    reads and writes. */
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
  /** Consumer-defined block; `kind` selects the implementation the host registered. */
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
] as const;

/* A type-level assertion rather than a `satisfies`, which isolatedDeclarations
   cannot emit on a const assertion. It fails to compile if the generator ever
   writes a data type the union does not define. */
type _DataTypesAreBlockTypes = (typeof DATA_TYPES)[number] extends BlockType ? true : never;
const _dataTypesCheck: _DataTypesAreBlockTypes = true;
void _dataTypesCheck;

/** A fresh block of the given type, carrying the starter payload every other
    kit inserts for it. */
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
          { blocks: [{ id: newId(), md: '', type: 'paragraph' }], ratio: 0.5 },
          { blocks: [{ id: newId(), md: '', type: 'paragraph' }], ratio: 0.5 },
        ],
      };
    case 'custom':
      return { id, type, kind: '', data: null };
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
      return { id, type, slices: [{ label: 'A', value: 3 }, { label: 'B', value: 5 }] };
    case 'diagram':
      return {
        id,
        type,
        nodes: [
          { id: 'start', kind: 'terminator', text: 'Start' },
          { id: 'work', kind: 'process', text: 'Work' },
          { id: 'done', kind: 'terminator', text: 'Done' },
        ],
        edges: [{ from: 'start', to: 'work' }, { from: 'work', to: 'done' }],
      };
    case 'sequence_diagram':
      return {
        id,
        type,
        participants: [{ id: 'a', name: 'Client' }, { id: 'b', name: 'Server' }],
        messages: [
          { from: 'a', text: 'request', to: 'b' },
          { from: 'b', kind: 'reply', text: 'response', to: 'a' },
        ],
      };
    case 'state_diagram':
      return {
        id,
        type,
        states: [
          { id: 'idle', initial: true, name: 'Idle' },
          { final: true, id: 'done', name: 'Done' },
        ],
        transitions: [{ from: 'idle', to: 'done', trigger: 'finish' }],
      };
    case 'node_table':
      return { id, type, title: 'Table', rows: [{ md: 'row' }] };
    case 'tree':
      return { id, type, nodes: [{ children: [{ title: 'child' }], title: 'root' }] };
    case 'timeline':
      return { id, type, items: [{ label: 'Start', on: '2026-01-01' }] };
    case 'chapter_header':
      return { id, type, title: 'Title' };
    case 'footnote':
      return { id, type, label: 'note-1', md: '' };
  }
}

/** Coarse runtime shape of one wire field — what document loading checks a
    field against. `'array'` says only that the field is an array; the
    element structs stay unchecked. A capitalised shape names a string-union
    helper type in `wire.ts`, and loading checks membership of the runtime
    list exported beside it. */
export type FieldShape =
  | 'string'
  | 'number'
  | 'boolean'
  | 'array'
  | 'unknown'
  | 'AdmonitionTone'
  | 'DiagramDirection'
  | 'ListStyle'
  | 'TimelineDirection';

/** One wire field of a kind: its name, whether serde may omit it, and its
    coarse runtime shape. */
export interface BlockFieldSpec {
  name: string;
  optional: boolean;
  shape: FieldShape;
}

/** Per kind, its wire fields in wire order — the table document loading
    validates a block against. */
export const BLOCK_FIELDS: Record<BlockType, readonly BlockFieldSpec[]> = {
  paragraph: [{ name: 'md', optional: false, shape: 'string' }],
  heading: [
    { name: 'level', optional: false, shape: 'number' },
    { name: 'md', optional: false, shape: 'string' },
  ],
  list_item: [
    { name: 'style', optional: false, shape: 'ListStyle' },
    { name: 'checked', optional: true, shape: 'boolean' },
    { name: 'indent', optional: false, shape: 'number' },
    { name: 'md', optional: false, shape: 'string' },
  ],
  quote: [{ name: 'md', optional: false, shape: 'string' }],
  divider: [],
  code: [
    { name: 'lang', optional: false, shape: 'string' },
    { name: 'code', optional: false, shape: 'string' },
  ],
  table: [
    { name: 'header', optional: false, shape: 'array' },
    { name: 'rows', optional: false, shape: 'array' },
  ],
  admonition: [
    { name: 'tone', optional: false, shape: 'AdmonitionTone' },
    { name: 'title', optional: false, shape: 'string' },
    { name: 'md', optional: false, shape: 'string' },
  ],
  columns: [{ name: 'columns', optional: false, shape: 'array' }],
  custom: [
    { name: 'kind', optional: false, shape: 'string' },
    { name: 'data', optional: false, shape: 'unknown' },
  ],
  image: [
    { name: 'src', optional: false, shape: 'string' },
    { name: 'alt', optional: false, shape: 'string' },
    { name: 'width', optional: true, shape: 'number' },
    { name: 'height', optional: true, shape: 'number' },
  ],
  video: [
    { name: 'src', optional: false, shape: 'string' },
    { name: 'poster', optional: true, shape: 'string' },
    { name: 'title', optional: true, shape: 'string' },
    { name: 'width', optional: true, shape: 'number' },
    { name: 'height', optional: true, shape: 'number' },
  ],
  math: [{ name: 'tex', optional: false, shape: 'string' }],
  bar_chart: [
    { name: 'title', optional: true, shape: 'string' },
    { name: 'x_label', optional: true, shape: 'string' },
    { name: 'y_label', optional: true, shape: 'string' },
    { name: 'categories', optional: false, shape: 'array' },
    { name: 'series', optional: false, shape: 'array' },
    { name: 'y_min', optional: true, shape: 'number' },
    { name: 'y_max', optional: true, shape: 'number' },
  ],
  line_chart: [
    { name: 'title', optional: true, shape: 'string' },
    { name: 'x_label', optional: true, shape: 'string' },
    { name: 'y_label', optional: true, shape: 'string' },
    { name: 'categories', optional: false, shape: 'array' },
    { name: 'series', optional: false, shape: 'array' },
    { name: 'y_min', optional: true, shape: 'number' },
    { name: 'y_max', optional: true, shape: 'number' },
    { name: 'points', optional: true, shape: 'array' },
    { name: 'point_labels', optional: true, shape: 'boolean' },
  ],
  pie_chart: [
    { name: 'title', optional: true, shape: 'string' },
    { name: 'slices', optional: false, shape: 'array' },
  ],
  diagram: [
    { name: 'direction', optional: true, shape: 'DiagramDirection' },
    { name: 'nodes', optional: false, shape: 'array' },
    { name: 'edges', optional: false, shape: 'array' },
  ],
  sequence_diagram: [
    { name: 'participants', optional: false, shape: 'array' },
    { name: 'messages', optional: false, shape: 'array' },
    { name: 'notes', optional: true, shape: 'array' },
  ],
  state_diagram: [
    { name: 'states', optional: false, shape: 'array' },
    { name: 'transitions', optional: false, shape: 'array' },
  ],
  node_table: [
    { name: 'title', optional: false, shape: 'string' },
    { name: 'rows', optional: false, shape: 'array' },
  ],
  tree: [{ name: 'nodes', optional: false, shape: 'array' }],
  timeline: [
    { name: 'title', optional: true, shape: 'string' },
    { name: 'direction', optional: true, shape: 'TimelineDirection' },
    { name: 'phases', optional: true, shape: 'array' },
    { name: 'items', optional: false, shape: 'array' },
  ],
  chapter_header: [
    { name: 'title', optional: false, shape: 'string' },
    { name: 'kicker', optional: true, shape: 'string' },
    { name: 'reading_time', optional: true, shape: 'string' },
    { name: 'updated', optional: true, shape: 'string' },
    { name: 'version', optional: true, shape: 'string' },
  ],
  footnote: [
    { name: 'label', optional: false, shape: 'string' },
    { name: 'md', optional: false, shape: 'string' },
  ],
};
