/**
 * The block kind registry, as the generators see it.
 *
 * The registry itself is Rust — `crates/forge-blocks/src/registry.rs`, where
 * the schema enum it describes also lives. Node cannot read that, so
 * `cargo run -p forge-blocks --bin dump-contract` writes it out as JSON and
 * this module loads the result. `cargo test -p forge-blocks` fails while the
 * JSON is stale, and `just check` fails while the TypeScript is.
 *
 * Nothing here decides anything: it loads, and it spells JSON values as the
 * TypeScript literals that produce them.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = dirname(dirname(dirname(fileURLToPath(import.meta.url))));

/** The authored source, for the banner. */
export const SOURCE_PATH = 'crates/forge-blocks/src/registry.rs';

/** The dump the generators read. */
export const REGISTRY_PATH = 'contract/blocks-registry.json';

/** The dump the emoji generator reads. */
export const EMOJI_PATH = 'contract/emoji.json';

/** The authored source of the emoji table, for its banner. */
export const EMOJI_SOURCE_PATH = 'crates/forge-blocks/src/emoji.rs';

/** The placeholder a starter carries where a fresh block id belongs. Written
    by `forge_blocks::export`; the generated constructor mints the real one. */
const ID_PLACEHOLDER = '$id';

const load = (path) => JSON.parse(readFileSync(join(REPO, path), 'utf8'));

const registry = load(REGISTRY_PATH);

/**
 * Every kind, in schema order: `{ type, label, is_data, markdown, doc,
 * fields: [{ name, ts, optional }], starter }`.
 */
export const kinds = registry.kinds;

/**
 * Every slash-palette row, in palette order: `{ id, label, hint }` plus either
 * an `insert` payload or a `wrap_columns` count.
 */
export const palette = registry.palette;

/** The emoji table as `[shortcode, glyph]` pairs, in shortcode order. */
export const emoji = load(EMOJI_PATH).emoji;

/** The names of the hand-written helper types a field list refers to. */
export function helperTypes(fields) {
  const names = new Set();
  for (const field of fields) for (const name of field.ts.match(/[A-Z]\w*/g) ?? []) names.add(name);
  return names;
}

/** A TypeScript string literal, single-quoted like the rest of the kit. */
export const quote = (text) => `'${text.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;

/** Whether `key` needs quoting to be an object key. */
const plainKey = (key) => /^[A-Za-z_$][\w$]*$/.test(key);

/**
 * A JSON value as the TypeScript expression that builds it, on one line.
 *
 * The starter's `$id` placeholders become `newId()` calls: a starter is a
 * template, and every block it makes needs an id of its own.
 */
export function expression(value) {
  if (value === ID_PLACEHOLDER) return 'newId()';
  if (value === null) return 'null';
  if (typeof value === 'string') return quote(value);
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (Array.isArray(value)) return `[${value.map(expression).join(', ')}]`;
  const pairs = Object.entries(value).map(
    ([key, item]) => `${plainKey(key) ? key : quote(key)}: ${expression(item)}`,
  );
  return `{ ${pairs.join(', ')} }`;
}

/**
 * A payload's fields in the wire order the registry declares, so an object
 * literal reads the way the schema does. Absent optional fields are left out,
 * as serde left them out. Nested objects keep the order the dump gave them.
 *
 * @param {object} head keys to put first — the `type` tag, where the literal
 *   states it rather than inheriting it from a `switch`.
 */
export function orderedPayload(payload, fields, head = {}) {
  const out = { ...head };
  for (const field of fields) {
    if (payload[field.name] !== undefined) out[field.name] = payload[field.name];
  }
  return out;
}

/** The same, as entries for [`entryLines`]. */
export function payloadEntries(payload, fields) {
  return Object.entries(orderedPayload(payload, fields)).map(([key, value]) => ({ key, value }));
}

const PRINT_WIDTH = 100;

/**
 * Render a value as TypeScript, on one line while it fits and broken over
 * several when it does not — the shape a formatter would settle on.
 *
 * @param {unknown} value the JSON value to write
 * @param {string} indent the leading whitespace of the line it opens on
 * @param {string} prefix what precedes it on that line
 * @param {string} suffix what follows it
 * @returns {string[]} one entry per line
 */
export function valueLines(value, indent, prefix = '', suffix = '', width = PRINT_WIDTH) {
  const inline = `${indent}${prefix}${expression(value)}${suffix}`;
  if (inline.length <= width) return [inline];
  if (Array.isArray(value)) {
    return [
      `${indent}${prefix}[`,
      ...value.flatMap((item) => valueLines(item, `${indent}  `, '', ',', width)),
      `${indent}]${suffix}`,
    ];
  }
  if (value && typeof value === 'object') {
    const entries = Object.entries(value).map(([key, item]) => ({ key, value: item }));
    return entryLines(entries, indent, prefix, suffix, width);
  }
  // A scalar too long to fit has nowhere to break.
  return [inline];
}

/**
 * Render an object literal from entries. An entry is one of:
 *
 * - `{ key, value }` — an ordinary property;
 * - `{ shorthand }` — a property that names a variable already in scope;
 * - `{ inline, lines }` — a property the caller renders itself, giving both
 *   its one-line form and a function from indent to its broken-out lines.
 */
export function entryLines(entries, indent, prefix = '', suffix = '', width = PRINT_WIDTH) {
  const keyOf = (entry) => (plainKey(entry.key) ? entry.key : quote(entry.key));
  const inlineOf = (entry) =>
    entry.shorthand ?? entry.inline ?? `${keyOf(entry)}: ${expression(entry.value)}`;

  const oneLine = `${indent}${prefix}{ ${entries.map(inlineOf).join(', ')} }${suffix}`;
  if (oneLine.length <= width) return [oneLine];

  const body = entries.flatMap((entry) => {
    if (entry.shorthand) return [`${indent}  ${entry.shorthand},`];
    if (entry.lines) return entry.lines(`${indent}  `);
    return valueLines(entry.value, `${indent}  `, `${keyOf(entry)}: `, ',', width);
  });
  return [`${indent}${prefix}{`, ...body, `${indent}}${suffix}`];
}

/** Wrap `lines` in a block comment at `indent`, in the kit's JSDoc style. */
export function docComment(lines, indent = '') {
  if (!lines.length) return [];
  if (lines.length === 1) return [`${indent}/** ${lines[0]} */`];
  const body = lines.map((line, i) => (i === 0 ? `${indent}/** ${line}` : `${indent}    ${line}`));
  body[body.length - 1] += ' */';
  return body;
}
