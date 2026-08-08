/**
 * Emit the block kind union, the data-kind list and the starter constructor.
 *
 * These are the three places the web kit used to restate the Rust schema: the
 * union of two dozen member shapes, the list of which kinds are data blocks,
 * and one starter payload per kind. All three come off the registry now, so a
 * kind added there arrives here without anyone editing TypeScript.
 *
 * What stays hand-written is `wire.ts`: the helper types the field list refers
 * to by name (`ChartSeries`, `TimelineItem`, …). They are structs on the Rust
 * side too, and a struct carries no per-kind policy to generate.
 */
import { bannerLines } from './banner.mjs';
import {
  REGISTRY_PATH,
  SOURCE_PATH,
  docComment,
  entryLines,
  helperTypes,
  kinds,
  payloadEntries,
  quote,
} from './blocks-source.mjs';

const INDENT = '  ';
const PRINT_WIDTH = 100;

/** One member of the union: the tag, then the fields in wire order. */
function unionMember(kind) {
  const pairs = [
    `type: ${quote(kind.type)}`,
    ...kind.fields.map((field) => `${field.name}${field.optional ? '?' : ''}: ${field.ts}`),
  ];
  const inline = `${INDENT}| { ${pairs.join('; ')} }`;
  const body =
    inline.length <= PRINT_WIDTH
      ? [inline]
      : [`${INDENT}| {`, ...pairs.map((p) => `${INDENT}    ${p};`), `${INDENT}  }`];
  return [...docComment(kind.doc, INDENT), ...body];
}

/** One arm of the starter constructor. */
function starterArm(kind) {
  // `type` is the narrowed switch subject, so the arm names it rather than
  // restating the tag it already holds.
  const entries = [
    { shorthand: 'id' },
    { shorthand: 'type' },
    ...payloadEntries(kind.starter, kind.fields),
  ];
  return [`    case ${quote(kind.type)}:`, ...entryLines(entries, '      ', 'return ', ';')];
}

/** The names the union refers to, imported from the hand-written `wire.ts`. */
function helperImport() {
  const names = new Set();
  for (const kind of kinds) for (const name of helperTypes(kind.fields)) names.add(name);
  const sorted = [...names].sort();
  const inline = `import type { ${sorted.join(', ')} } from './wire';`;
  if (inline.length <= PRINT_WIDTH) return [inline];
  return ['import type {', ...sorted.map((name) => `  ${name},`), "} from './wire';"];
}

/** The whole file. */
export function renderBlocksTypes() {
  const lines = [
    `/* ${bannerLines(SOURCE_PATH, REGISTRY_PATH).join('\n   ')} */`,
    "import { newId } from './id';",
    ...helperImport(),
    '',
    "/** Every block the schema defines: the block's own `id`, then the `type` tag",
    '    and the fields beside it that make up the wire shape `crates/forge-blocks`',
    '    reads and writes. */',
    'export type Block = { id: string } & (',
    ...kinds.flatMap(unionMember),
    ');',
    '',
    "export type BlockType = Block['type'];",
    '',
    '/** Data-block types: rendered from structured fields and edited as raw JSON',
    '    source rather than through a bespoke editor. Mirrors Rust `is_data()`',
    '    (footnote is text-bearing, math edits its `tex` as plain text on web). */',
    'export const DATA_TYPES = [',
    ...kinds.filter((kind) => kind.is_data).map((kind) => `  ${quote(kind.type)},`),
    '] as const;',
    '',
    '/* A type-level assertion rather than a `satisfies`, which isolatedDeclarations',
    '   cannot emit on a const assertion. It fails to compile if the generator ever',
    '   writes a data type the union does not define. */',
    'type _DataTypesAreBlockTypes = (typeof DATA_TYPES)[number] extends BlockType ? true : never;',
    'const _dataTypesCheck: _DataTypesAreBlockTypes = true;',
    'void _dataTypesCheck;',
    '',
    '/** A fresh block of the given type, carrying the starter payload every other',
    '    kit inserts for it. */',
    'export function createBlock(type: BlockType): Block {',
    '  const id = newId();',
    '  switch (type) {',
    ...kinds.flatMap(starterArm),
    '  }',
    '}',
  ];
  return `${lines.join('\n')}\n`;
}
