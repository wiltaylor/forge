/**
 * The block kind registry, as the generators see it.
 *
 * The registry itself is Rust — `crates/forge-blocks/src/registry.rs`, where
 * the schema enum it describes also lives. Node cannot read that, so
 * `cargo run -p forge-blocks --bin dump-contract` writes it out as JSON and
 * this module loads the result — refusing a dump older than the Rust file it
 * came from, so a stale one cannot pass for current.
 *
 * Nothing here decides anything: it loads, and it spells JSON values as the
 * TypeScript literals that produce them.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = dirname(dirname(dirname(fileURLToPath(import.meta.url))));

/** The authored registry, for the banner. */
export const REGISTRY_SOURCE_PATH = 'crates/forge-blocks/src/registry.rs';

/** The dump the kind generators read. */
export const REGISTRY_PATH = 'contract/blocks-registry.json';

/** The authored emoji table, for its banner. */
export const EMOJI_SOURCE_PATH = 'crates/forge-blocks/src/emoji.rs';

/** The dump the emoji generator reads. */
export const EMOJI_PATH = 'contract/emoji.json';

/** What the banner says about a dump: which recipe rewrites it. */
export const via = (path) => `${path}   (\`just generate-blocks\` rewrites it)`;

/** The placeholder a starter carries where a fresh block id belongs. Written
    by `forge_blocks::export`; the generated constructor mints the real one. */
const ID_PLACEHOLDER = '$id';

const read = (path) => readFileSync(join(REPO, path), 'utf8');

/**
 * The digest `forge_blocks::export::digest` writes beside a dump — FNV-1a over
 * the source text, carriage returns dropped. Keep the two implementations
 * identical.
 */
function digest(source) {
  let hash = 0xcbf29ce484222325n;
  for (const byte of Buffer.from(source, 'utf8')) {
    if (byte === 0x0d) continue;
    hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x00000100000001b3n);
  }
  return `fnv1a64:${hash.toString(16).padStart(16, '0')}`;
}

/**
 * A dump, refused if the Rust file it came from has changed since.
 *
 * Without this the Node generators would happily rewrite the TypeScript from a
 * stale dump, and `just check` — which is Node only, on purpose — would call
 * the result up to date. The digest is what lets it see the whole chain.
 */
function load(path, sourcePath) {
  const dump = JSON.parse(read(path));
  const wanted = digest(read(sourcePath));
  if (dump.source_digest !== wanted) {
    throw new Error(
      `${path} was written from an older ${sourcePath}.\n` +
        'Run `just generate-blocks` and commit the result.',
    );
  }
  return dump;
}

const registry = load(REGISTRY_PATH, REGISTRY_SOURCE_PATH);

/**
 * Every kind, in schema order: `{ type, is_data, doc,
 * fields: [{ name, ts, optional }], starter }`.
 */
export const kinds = registry.kinds;

/**
 * Every slash-palette row, in palette order: `{ id, label, hint }` plus either
 * an `insert` payload or a `wrap_columns` count.
 */
export const palette = registry.palette;

/** The emoji table as `[shortcode, glyph]` pairs, in shortcode order. */
export const emoji = load(EMOJI_PATH, EMOJI_SOURCE_PATH).emoji;

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

/** The width the emitted TypeScript wraps at. */
export const PRINT_WIDTH = 100;

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
