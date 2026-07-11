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
  }
}

/** An empty document holding a single empty paragraph (the editor invariant:
    a document is never blockless). */
export function emptyDocument(): BlockDocument {
  return { version: DOC_VERSION, blocks: [createBlock('paragraph')] };
}
