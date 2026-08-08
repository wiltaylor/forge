/* The block document schema — the JSON interchange contract shared verbatim
   with crates/forge-blocks (Rust). Any shape change must land on both sides
   (see crates/forge-blocks/tests/schema.rs for the literal fixtures).

   Three parts. The kind union, the data-kind list and the starter constructor
   are generated from the Rust kind registry (`types.gen.ts`). The field types
   they are built from are hand-written (`wire.ts`). Everything a kit needs
   *about* a block — the document wrapper, the predicates, the custom-block
   interface — is here. */
import type { JSX } from 'solid-js';
import type { IconComponent } from '@forge/ui';
import type { Block } from './types.gen';
import { DATA_TYPES, createBlock } from './types.gen';

export * from './wire';
export * from './types.gen';
export { newId } from './id';

export const DOC_VERSION = 1;

export interface BlockDocument {
  version: typeof DOC_VERSION;
  blocks: Block[];
}

export type TextBlock = Extract<Block, { md: string }>;

/** A block without its id — distributed over the union (plain `Omit`
    collapses union members to their common properties). */
export type BlockData = Block extends infer B ? (B extends Block ? Omit<B, 'id'> : never) : never;

/** Blocks whose `md` body is edited with the shared text keyboard model. */
export function isTextBlock(b: Block): b is TextBlock {
  return 'md' in b;
}

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

/** An empty document holding a single empty paragraph (the editor invariant:
    a document is never blockless). */
export function emptyDocument(): BlockDocument {
  return { version: DOC_VERSION, blocks: [createBlock('paragraph')] };
}
