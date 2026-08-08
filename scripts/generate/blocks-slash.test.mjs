/**
 * Tests for the slash palette generator.
 *
 * The point of generating this list is that three kits cannot offer three
 * different ones, so what these assert is correspondence: one row out per row
 * in, in the same order, saying the same thing.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import { REGISTRY_PATH, SOURCE_PATH, palette } from './blocks-source.mjs';
import { renderBlocksSlash } from './blocks-slash.mjs';

const ts = renderBlocksSlash();
const list = ts.slice(ts.indexOf('export const SLASH_BUILTINS'));

/**
 * The rows of the emitted array, one string each. A row is either a single
 * line or a block ending in `  },`, so splitting on that is enough — the
 * payloads nested inside sit at a deeper indent.
 */
function emittedRows() {
  const body = list.slice(list.indexOf('[\n') + 2, list.lastIndexOf('\n];'));
  const rows = [];
  let current = [];
  for (const line of body.split('\n')) {
    current.push(line);
    if (line === '  },' || (line.startsWith('  {') && line.endsWith('},'))) {
      rows.push(current.join('\n'));
      current = [];
    }
  }
  assert.equal(current.length, 0, 'a row was left unclosed');
  return rows;
}

const rows = emittedRows();

/** The text of one row, found by the id it opens with. */
function rowText(id) {
  const found = rows.find((row) => row.includes(`id: '${id}'`));
  assert.ok(found, `no row has the id ${id}`);
  return found;
}

test('the file opens with a header naming it generated and pointing at the source', () => {
  const head = ts.slice(0, ts.indexOf('*/'));
  for (const line of bannerLines(SOURCE_PATH, REGISTRY_PATH)) {
    assert.ok(head.includes(line), `header is missing: ${line}`);
  }
});

test('one row per registry row, in palette order', () => {
  const ids = rows.map((row) => row.match(/id: '([a-z0-9_]+)'/)[1]);
  assert.deepEqual(
    ids,
    palette.map((row) => row.id),
  );
});

test('a row reads as the registry labels it', () => {
  for (const row of palette) {
    assert.ok(rowText(row.id).includes(`label: '${row.label}'`), `${row.id} is labelled wrongly`);
  }
});

test('a row carries a hint exactly when the registry gives it one', () => {
  for (const row of palette) {
    const text = rowText(row.id);
    if (row.hint === null) assert.ok(!text.includes('hint:'), `${row.id} invented a hint`);
    else assert.ok(text.includes(`hint: '${row.hint}'`), `${row.id} lost its hint`);
  }
});

test('a row either makes a block or wraps in columns, never both', () => {
  for (const row of palette) {
    const text = rowText(row.id);
    const wraps = row.wrap_columns !== undefined;
    assert.equal(text.includes('columns:'), wraps, `${row.id} disagrees about wrapping`);
    assert.equal(text.includes('make:'), !wraps, `${row.id} disagrees about inserting`);
    if (wraps) assert.ok(text.includes(`columns: ${row.wrap_columns}`));
    else assert.ok(text.includes(`type: '${row.insert.type}'`), `${row.id} makes another kind`);
  }
});

test('the column counts the rows use are the ones the interface allows', () => {
  const allowed = ts.match(/columns\?: ([\d |]+);/)[1].split(' | ');
  const used = [...new Set(palette.filter((r) => r.wrap_columns).map((r) => `${r.wrap_columns}`))];
  assert.deepEqual(allowed.sort(), used.sort());
});
