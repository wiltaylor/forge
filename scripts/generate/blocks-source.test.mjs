/**
 * Tests for the loader and the value printer the block generators share.
 *
 * Everything the three generators emit passes through `expression`,
 * `valueLines` and `entryLines`, so these assert the two decisions those make:
 * what a JSON value spells as, and where a line breaks.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { entryLines, expression, orderedPayload, valueLines } from './blocks-source.mjs';
import { PRINT_WIDTH } from './ts.mjs';

test('a value spells as the TypeScript that builds it', () => {
  assert.equal(expression('a'), "'a'");
  assert.equal(expression(3), '3');
  assert.equal(expression(true), 'true');
  assert.equal(expression(null), 'null');
  assert.equal(expression([1, 'two']), "[1, 'two']");
  assert.equal(expression({ a: 1, 'x-y': 2 }), "{ a: 1, 'x-y': 2 }");
});

test('the id placeholder becomes a fresh id, not a constant', () => {
  assert.equal(expression({ id: '$id', type: 'paragraph' }), "{ id: newId(), type: 'paragraph' }");
});

test('a value stays on one line while it fits', () => {
  assert.deepEqual(valueLines([1, 2], '  ', 'x: ', ','), ['  x: [1, 2],']);
});

test('an array too long for the line breaks one element per line', () => {
  const long = Array.from({ length: 12 }, (_, i) => `element-${i}`);
  const lines = valueLines(long, '  ', 'x: ', ',');
  assert.equal(lines[0], '  x: [');
  assert.equal(lines.at(-1), '  ],');
  assert.equal(lines.length, long.length + 2);
  for (const line of lines) assert.ok(line.length <= PRINT_WIDTH, `too wide: ${line}`);
});

test('a nested value breaks only as far as it has to', () => {
  // The outer object breaks; the inner one still fits, so it stays whole.
  const value = { one: Array.from({ length: 8 }, () => 'padding'), two: { a: 1 } };
  const lines = valueLines(value, '', '', ';');
  assert.ok(lines.length > 1);
  assert.ok(lines.some((line) => line.includes('two: { a: 1 },')));
});

test('a shorthand entry names a variable rather than restating it', () => {
  const entries = [{ shorthand: 'id' }, { key: 'md', value: '' }];
  assert.deepEqual(entryLines(entries, '', 'return ', ';'), ["return { id, md: '' };"]);
});

test('a caller-rendered entry supplies both its forms', () => {
  const entry = { inline: 'make: () => (1)', lines: (indent) => [`${indent}make: () => (1),`] };
  const padding = { key: 'label', value: 'x'.repeat(PRINT_WIDTH) };
  assert.deepEqual(entryLines([entry], '', '', ','), ['{ make: () => (1) },']);
  assert.ok(entryLines([padding, entry], '', '', ',').includes('  make: () => (1),'));
});

test('a payload is ordered by the wire fields, absent ones left out', () => {
  const fields = [{ name: 'style' }, { name: 'checked' }, { name: 'md' }];
  const payload = { md: 'x', style: 'bullet' };
  assert.deepEqual(orderedPayload(payload, fields, { type: 'list_item' }), {
    type: 'list_item',
    style: 'bullet',
    md: 'x',
  });
});
