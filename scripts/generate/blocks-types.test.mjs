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
import { REGISTRY_PATH, REGISTRY_SOURCE_PATH, helperTypes, kinds, via } from './blocks-source.mjs';
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
  for (const line of bannerLines(REGISTRY_SOURCE_PATH, via(REGISTRY_PATH))) {
    assert.ok(head.includes(line), `header is missing: ${line}`);
  }
  assert.ok(head.includes(REGISTRY_SOURCE_PATH));
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

const fieldTable = section('export const BLOCK_FIELDS', '\n};');

/** The table text for one kind: from its key to the next kind's key. */
function fieldRowsFor(type) {
  const open = new RegExp(`^  ${type}: \\[`, 'm').exec(fieldTable);
  assert.ok(open, `the field table omits ${type}`);
  const rest = fieldTable.slice(open.index + 2 + type.length);
  const next = /^  [a-z_]+: \[/m.exec(rest);
  return rest.slice(0, next ? next.index : undefined);
}

test('the field table has one entry per kind, in registry order', () => {
  const emitted = (fieldTable.match(/^  ([a-z_]+): \[/gm) ?? []).map((m) => m.trim().split(':')[0]);
  assert.deepEqual(
    emitted,
    kinds.map((kind) => kind.type),
  );
});

test('a field row states its name and the optionality serde gives it', () => {
  for (const kind of kinds) {
    const rows = fieldRowsFor(kind.type);
    for (const field of kind.fields) {
      const wanted = `{ name: '${field.name}', optional: ${field.optional}, shape: `;
      assert.ok(rows.includes(wanted), `${kind.type}.${field.name} row does not open ${wanted}`);
    }
    const count = (rows.match(/name: '/g) ?? []).length;
    assert.equal(count, kind.fields.length, `${kind.type} has a row the registry does not`);
  }
});

test('a field checks as an array exactly when its wire type is one', () => {
  for (const kind of kinds) {
    const rows = fieldRowsFor(kind.type);
    for (const field of kind.fields) {
      const row = new RegExp(`name: '${field.name}',[^}]*shape: '([A-Za-z]+)'`).exec(rows);
      assert.ok(row, `${kind.type}.${field.name} has no shape`);
      assert.equal(
        row[1] === 'array',
        field.ts.endsWith('[]'),
        `${kind.type}.${field.name} (${field.ts}) checks as ${row[1]}`,
      );
    }
  }
});

test('a scalar helper type is its own shape, for enum-membership checks', () => {
  // ListStyle, AdmonitionTone, the direction enums: string unions in wire.ts,
  // named so loading can check membership rather than admit any string.
  assert.ok(fieldRowsFor('list_item').includes("{ name: 'style', optional: false, shape: 'ListStyle' }"));
  assert.ok(fieldRowsFor('admonition').includes("{ name: 'tone', optional: false, shape: 'AdmonitionTone' }"));
});

test('the FieldShape union carries every shape the table uses, and no other', () => {
  const union = section('export type FieldShape =', ';\n');
  const declared = new Set((union.match(/'[A-Za-z]+'/g) ?? []).map((q) => q.slice(1, -1)));
  const used = new Set(['string', 'number', 'boolean', 'array', 'unknown']);
  for (const kind of kinds) {
    for (const field of kind.fields) {
      const row = new RegExp(`name: '${field.name}',[^}]*shape: '([A-Za-z]+)'`).exec(
        fieldRowsFor(kind.type),
      );
      used.add(row[1]);
    }
  }
  assert.deepEqual([...declared].sort(), [...used].sort());
});

test('a custom payload stays unchecked', () => {
  assert.ok(fieldRowsFor('custom').includes("{ name: 'data', optional: false, shape: 'unknown' }"));
});

test('a nested block gets a fresh id rather than the placeholder', () => {
  // The `columns` starter holds blocks. Committing the id the dump recorded
  // would give every document the same one.
  assert.ok(!ts.includes('$id'), 'the id placeholder reached the output');
  assert.ok(starters.includes('id: newId()'), 'no nested block mints an id');
});
