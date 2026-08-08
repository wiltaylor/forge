/**
 * Tests for the block type generator.
 *
 * These assert relations between the registry and the emitted TypeScript —
 * which kinds reach the union, which fields are optional there, which arms the
 * constructor grows. They never restate a payload: `just check` already proves
 * the committed file matches the registry, so a literal here would only be a
 * second copy of it.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import { REGISTRY_PATH, SOURCE_PATH, helperTypes, kinds } from './blocks-source.mjs';
import { renderBlocksTypes } from './blocks-types.mjs';

const ts = renderBlocksTypes();

/** The text between `open` and the matching line that starts with `close`. */
function section(open, close) {
  const from = ts.indexOf(open);
  assert.notEqual(from, -1, `the file has no ${open}`);
  const to = ts.indexOf(close, from);
  assert.notEqual(to, -1, `${open} is never closed by ${close}`);
  return ts.slice(from, to);
}

const union = section('export type Block = { id: string } & (', '\n);');
const starters = section('export function createBlock', '\n}\n');

test('the file opens with a header naming it generated and pointing at the source', () => {
  const head = ts.slice(0, ts.indexOf('*/'));
  for (const line of bannerLines(SOURCE_PATH, REGISTRY_PATH)) {
    assert.ok(head.includes(line), `header is missing: ${line}`);
  }
  assert.ok(head.includes(SOURCE_PATH));
  assert.ok(head.includes(REGISTRY_PATH), 'the header does not say which file it read');
});

test('every kind the registry defines is a member of the union', () => {
  for (const kind of kinds) {
    assert.ok(union.includes(`type: '${kind.type}'`), `the union omits ${kind.type}`);
  }
  const members = union.match(/type: '[a-z_]+'/g) ?? [];
  assert.equal(members.length, kinds.length, 'the union has a member the registry does not');
});

test('a field is optional in the union exactly when serde omits it', () => {
  for (const kind of kinds) {
    for (const field of kind.fields) {
      const wanted = `${field.name}${field.optional ? '?' : ''}: ${field.ts}`;
      assert.ok(union.includes(wanted), `${kind.type}.${field.name} is not written as ${wanted}`);
    }
  }
});

test('the union imports every helper type its fields name, and no other', () => {
  const imported = section('import type', "from './wire'")
    .replace(/import type\s*\{?/, '')
    .split(/[,{}\s]+/)
    .filter(Boolean);
  const named = new Set();
  for (const kind of kinds) for (const name of helperTypes(kind.fields)) named.add(name);
  assert.deepEqual([...imported].sort(), [...named].sort());
});

test('the data-kind list is the registry data kinds, in registry order', () => {
  const list = section('export const DATA_TYPES = [', '] as const;');
  const emitted = (list.match(/'[a-z_]+'/g) ?? []).map((q) => q.slice(1, -1));
  assert.deepEqual(
    emitted,
    kinds.filter((kind) => kind.is_data).map((kind) => kind.type),
  );
});

test('the constructor has one arm per kind, in registry order', () => {
  const arms = (starters.match(/case '([a-z_]+)':/g) ?? []).map((c) => c.slice(6, -2));
  assert.deepEqual(
    arms,
    kinds.map((kind) => kind.type),
  );
});

test('an arm states the fields its starter serializes, and only those', () => {
  for (const kind of kinds) {
    const from = starters.indexOf(`case '${kind.type}':`);
    const to = starters.indexOf('    case ', from + 1);
    const arm = starters.slice(from, to === -1 ? undefined : to);
    for (const field of kind.fields) {
      const present = kind.starter[field.name] !== undefined;
      assert.equal(
        new RegExp(`\\b${field.name}:`).test(arm),
        present,
        `${kind.type}.${field.name} ${present ? 'is missing from' : 'should not be in'} its arm`,
      );
    }
  }
});

test('a nested block gets a fresh id rather than the placeholder', () => {
  // The `columns` starter holds blocks. Committing the id the dump recorded
  // would give every document the same one.
  assert.ok(!ts.includes('$id'), 'the id placeholder reached the output');
  assert.ok(starters.includes('id: newId()'), 'no nested block mints an id');
});
