/**
 * Emit the web kit's slash-palette rows.
 *
 * The two Rust kits read `forge_blocks::palette_rows()` directly. The web kit
 * cannot, so its rows come through here — same ids, same labels, same order,
 * same starter payloads. That is the whole point: three palettes cannot offer
 * three different kind lists if only one of them is authored.
 *
 * What the kits still decide for themselves is context: whether to show the
 * column rows (a block already inside a column cannot hold more columns) and
 * which custom kinds the host registered.
 */
import { bannerLines } from './banner.mjs';
import {
  REGISTRY_PATH,
  REGISTRY_SOURCE_PATH,
  entryLines,
  expression,
  kinds,
  orderedPayload,
  palette,
  valueLines,
  via,
} from './blocks-source.mjs';

/** The kind entry a palette payload belongs to. */
const entryFor = (payload) => kinds.find((kind) => kind.type === payload.type);

/** The `make` property: a thunk returning the row's payload. */
function makeEntry(insert) {
  const payload = orderedPayload(insert, entryFor(insert).fields, { type: insert.type });
  return {
    inline: `make: () => (${expression(payload)})`,
    lines: (indent) => valueLines(payload, indent, 'make: () => (', '),'),
  };
}

/** One row as an object literal. */
function row(item) {
  const entries = [
    { key: 'id', value: item.id },
    { key: 'label', value: item.label },
    ...(item.hint === null ? [] : [{ key: 'hint', value: item.hint }]),
    item.wrap_columns === undefined
      ? makeEntry(item.insert)
      : { key: 'columns', value: item.wrap_columns },
  ];
  return entryLines(entries, '  ', '', ',');
}

/** The whole file. */
export function renderBlocksSlash() {
  const columnCounts = [
    ...new Set(palette.filter((r) => r.wrap_columns !== undefined).map((r) => r.wrap_columns)),
  ];
  const lines = [
    `/* ${bannerLines(REGISTRY_SOURCE_PATH, via(REGISTRY_PATH)).join('\n   ')} */`,
    "import type { BlockData } from './types';",
    '',
    '/** One built-in row of the `/` palette. A row either makes a block or wraps',
    '    the block it was typed in — never both. */',
    'export interface SlashBuiltin {',
    '  /** The registry id of the row, stable across kits. */',
    '  id: string;',
    '  label: string;',
    '  /** The markdown shortcut that produces the same block. */',
    '  hint?: string;',
    '  /** Wrap the block into this many columns instead of replacing it. */',
    `  columns?: ${columnCounts.join(' | ')};`,
    '  /** The block the row inserts, without its id. */',
    '  make?: () => BlockData;',
    '}',
    '',
    '/** Every built-in row, in the order every kit lists them. */',
    'export const SLASH_BUILTINS: SlashBuiltin[] = [',
    ...palette.flatMap(row),
    '];',
  ];
  return `${lines.join('\n')}\n`;
}
