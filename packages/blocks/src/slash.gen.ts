/* GENERATED FILE — do not edit by hand.
   Source:     crates/forge-blocks/src/registry.rs
   Read from:  contract/blocks-registry.json   (`just generate-blocks` rewrites it)
   Regenerate: just generate   (`just check` fails while this file is stale) */
import type { BlockData } from './types';

/** One built-in row of the `/` palette. A row either makes a block or wraps
    the block it was typed in — never both. */
export interface SlashBuiltin {
  /** The registry id of the row, stable across kits. */
  id: string;
  label: string;
  /** The markdown shortcut that produces the same block. */
  hint?: string;
  /** Wrap the block into this many columns instead of replacing it. */
  columns?: 2 | 3;
  /** The block the row inserts, without its id. */
  make?: () => BlockData;
}

/** Every built-in row, in the order every kit lists them. */
export const SLASH_BUILTINS: SlashBuiltin[] = [
  { id: 'text', label: 'Text', make: () => ({ type: 'paragraph', md: '' }) },
  { id: 'h1', label: 'Heading 1', hint: '#', make: () => ({ type: 'heading', level: 1, md: '' }) },
  { id: 'h2', label: 'Heading 2', hint: '##', make: () => ({ type: 'heading', level: 2, md: '' }) },
  {
    id: 'h3',
    label: 'Heading 3',
    hint: '###',
    make: () => ({ type: 'heading', level: 3, md: '' }),
  },
  {
    id: 'h4',
    label: 'Heading 4',
    hint: '####',
    make: () => ({ type: 'heading', level: 4, md: '' }),
  },
  {
    id: 'bullet',
    label: 'Bullet list',
    hint: '-',
    make: () => ({ type: 'list_item', style: 'bullet', indent: 0, md: '' }),
  },
  {
    id: 'number',
    label: 'Numbered list',
    hint: '1.',
    make: () => ({ type: 'list_item', style: 'number', indent: 0, md: '' }),
  },
  {
    id: 'todo',
    label: 'To-do list',
    hint: '[]',
    make: () => ({ type: 'list_item', style: 'todo', checked: false, indent: 0, md: '' }),
  },
  { id: 'quote', label: 'Quote', hint: '>', make: () => ({ type: 'quote', md: '' }) },
  { id: 'divider', label: 'Divider', hint: '---', make: () => ({ type: 'divider' }) },
  { id: 'code', label: 'Code', hint: '```', make: () => ({ type: 'code', lang: '', code: '' }) },
  {
    id: 'table',
    label: 'Table',
    make: () => ({ type: 'table', header: ['', '', ''], rows: [['', '', ''], ['', '', '']] }),
  },
  {
    id: 'callout',
    label: 'Callout',
    hint: ':::',
    make: () => ({ type: 'admonition', tone: 'info', title: '', md: '' }),
  },
  { id: 'image', label: 'Image', hint: '![]', make: () => ({ type: 'image', src: '', alt: '' }) },
  { id: 'video', label: 'Video', hint: 'embed', make: () => ({ type: 'video', src: '' }) },
  { id: 'math', label: 'Math', hint: '$$', make: () => ({ type: 'math', tex: '' }) },
  {
    id: 'bar_chart',
    label: 'Bar chart',
    make: () => ({
      type: 'bar_chart',
      categories: ['A', 'B', 'C'],
      series: [{ name: 'Series 1', values: [3, 5, 4] }],
    }),
  },
  {
    id: 'line_chart',
    label: 'Line chart',
    make: () => ({
      type: 'line_chart',
      categories: ['A', 'B', 'C'],
      series: [{ name: 'Series 1', values: [3, 5, 4] }],
    }),
  },
  {
    id: 'pie_chart',
    label: 'Pie chart',
    make: () => ({
      type: 'pie_chart',
      slices: [{ label: 'A', value: 3 }, { label: 'B', value: 5 }],
    }),
  },
  {
    id: 'diagram',
    label: 'Diagram',
    hint: 'flow',
    make: () => ({
      type: 'diagram',
      nodes: [
        { id: 'start', kind: 'terminator', text: 'Start' },
        { id: 'work', kind: 'process', text: 'Work' },
        { id: 'done', kind: 'terminator', text: 'Done' },
      ],
      edges: [{ from: 'start', to: 'work' }, { from: 'work', to: 'done' }],
    }),
  },
  {
    id: 'sequence_diagram',
    label: 'Sequence diagram',
    make: () => ({
      type: 'sequence_diagram',
      participants: [{ id: 'a', name: 'Client' }, { id: 'b', name: 'Server' }],
      messages: [
        { from: 'a', text: 'request', to: 'b' },
        { from: 'b', kind: 'reply', text: 'response', to: 'a' },
      ],
    }),
  },
  {
    id: 'state_diagram',
    label: 'State diagram',
    make: () => ({
      type: 'state_diagram',
      states: [
        { id: 'idle', initial: true, name: 'Idle' },
        { final: true, id: 'done', name: 'Done' },
      ],
      transitions: [{ from: 'idle', to: 'done', trigger: 'finish' }],
    }),
  },
  {
    id: 'node_table',
    label: 'Node table',
    make: () => ({ type: 'node_table', title: 'Table', rows: [{ md: 'row' }] }),
  },
  {
    id: 'tree',
    label: 'Tree',
    make: () => ({ type: 'tree', nodes: [{ children: [{ title: 'child' }], title: 'root' }] }),
  },
  {
    id: 'timeline',
    label: 'Timeline',
    make: () => ({ type: 'timeline', items: [{ label: 'Start', on: '2026-01-01' }] }),
  },
  {
    id: 'chapter_header',
    label: 'Chapter header',
    make: () => ({ type: 'chapter_header', title: 'Title' }),
  },
  {
    id: 'footnote',
    label: 'Footnote',
    hint: '[^]',
    make: () => ({ type: 'footnote', label: 'note-1', md: '' }),
  },
  { id: 'col2', label: '2 columns', columns: 2 },
  { id: 'col3', label: '3 columns', columns: 3 },
];
