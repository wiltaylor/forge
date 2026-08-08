/* Loading untrusted JSON as a block document — the web kit's counterpart to
   what the Rust kits get from serde deserialisation plus
   `Document::normalize` (crates/forge-blocks/src/schema.rs).

   Three promises. A document whose version this editor does not recognise is
   refused before its blocks are read. A malformed block is refused with a
   message naming its path, its type and the field at fault, rather than
   silently cast. A loaded document carries the editor invariants: never
   blockless, and columns hold no nested columns and no empty cells.

   Validation is presence and coarse shape, driven by the generated
   `BLOCK_FIELDS` table so the kind list has one author. Element structs
   inside arrays (chart series, diagram nodes, …) stay unchecked, as does the
   range of numeric fields — matching what the renderers tolerate. Fields the
   schema does not declare are dropped, as serde drops them. */
import type { Block, BlockDocument, BlockFieldSpec, FieldShape } from './types';
import { BLOCK_FIELDS, DOC_VERSION, createBlock } from './types';

/** What `loadDocument` throws; `message` says what was wrong and where. */
export class DocumentLoadError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'DocumentLoadError';
  }
}

function fail(message: string): never {
  throw new DocumentLoadError(message);
}

const isRecord = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

/** A value's kind, for messages: `null`, `an array`, `a string`, … */
function describe(v: unknown): string {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'an array';
  return `a ${typeof v}`;
}

const SHAPE_CHECK: Record<FieldShape, (v: unknown) => boolean> = {
  string: (v) => typeof v === 'string',
  number: (v) => typeof v === 'number' && Number.isFinite(v),
  boolean: (v) => typeof v === 'boolean',
  array: (v) => Array.isArray(v),
  unknown: () => true,
};

const SHAPE_WANTED: Record<FieldShape, string> = {
  string: 'a string',
  number: 'a number',
  boolean: 'a boolean',
  array: 'an array',
  unknown: 'anything',
};

/** One field checked and copied onto `out`; a dropped field copies nothing. */
function readField(
  value: Record<string, unknown>,
  field: BlockFieldSpec,
  out: Record<string, unknown>,
  at: string,
): void {
  const v = value[field.name];
  if (v === undefined) {
    if (!field.optional) fail(`${at}: missing required field "${field.name}"`);
    return;
  }
  // A null optional field reads as absent, the way serde reads Option::None.
  if (v === null && field.optional && field.shape !== 'unknown') return;
  if (!SHAPE_CHECK[field.shape](v)) {
    fail(`${at}: field "${field.name}" must be ${SHAPE_WANTED[field.shape]}, got ${describe(v)}`);
  }
  out[field.name] = v;
}

/** One block checked and rebuilt from its declared fields only. */
function readBlock(value: unknown, path: string): Block {
  if (!isRecord(value)) fail(`${path}: a block must be an object, got ${describe(value)}`);
  const type = value.type;
  if (typeof type !== 'string') fail(`${path}: a block must carry a string "type"`);
  const fields = (BLOCK_FIELDS as Record<string, readonly BlockFieldSpec[]>)[type];
  if (!fields) fail(`${path}: unknown block type "${type}"`);
  const at = `${path} (${type})`;
  if (typeof value.id !== 'string') fail(`${at}: a block must carry a string "id"`);

  const out: Record<string, unknown> = { id: value.id, type };
  for (const field of fields) readField(value, field, out, at);
  if (type === 'columns') {
    out.columns = (out.columns as unknown[]).map((col, i) =>
      readColumn(col, `${path}.columns[${i}]`),
    );
  }
  return out as Block;
}

/** One column cell: a ratio and the blocks it holds, each checked in turn. */
function readColumn(value: unknown, path: string): { ratio: number; blocks: Block[] } {
  if (!isRecord(value)) fail(`${path}: a column must be an object, got ${describe(value)}`);
  const { ratio, blocks } = value;
  if (typeof ratio !== 'number' || !Number.isFinite(ratio)) {
    fail(`${path}: a column must carry a number "ratio"`);
  }
  if (!Array.isArray(blocks)) fail(`${path}: a column must carry a "blocks" array`);
  return {
    ratio,
    blocks: (blocks as unknown[]).map((b, i) => readBlock(b, `${path}.blocks[${i}]`)),
  };
}

/** The invariants `Document::normalize` restores on the Rust side: never
    blockless, and columns hold no nested columns and no empty cells. */
function normalize(blocks: Block[]): Block[] {
  const out = blocks.map((block) => {
    if (block.type !== 'columns') return block;
    const columns = block.columns.map((col) => {
      const kept = col.blocks.filter((b) => b.type !== 'columns');
      return { ...col, blocks: kept.length ? kept : [createBlock('paragraph')] };
    });
    return { ...block, columns };
  });
  return out.length ? out : [createBlock('paragraph')];
}

/**
 * Read a document from parsed JSON of unknown provenance.
 *
 * Returns a fresh, normalised `BlockDocument`; throws [`DocumentLoadError`]
 * for an unrecognised version or a malformed block. The version is checked
 * first — a future version may change what a block is, and "wrong version"
 * is the message worth acting on.
 */
export function loadDocument(input: unknown): BlockDocument {
  if (!isRecord(input)) fail(`a document must be an object, got ${describe(input)}`);
  if (input.version !== DOC_VERSION) {
    fail(
      `unrecognised document version ${JSON.stringify(input.version) ?? 'undefined'}; ` +
        `this editor reads version ${DOC_VERSION}`,
    );
  }
  if (!Array.isArray(input.blocks)) fail('a document must carry a "blocks" array');
  const blocks = (input.blocks as unknown[]).map((b, i) => readBlock(b, `blocks[${i}]`));
  return { version: DOC_VERSION, blocks: normalize(blocks) };
}
