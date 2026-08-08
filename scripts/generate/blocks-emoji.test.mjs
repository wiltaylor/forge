/**
 * Tests for the emoji table generator.
 *
 * Eight hundred and thirty-six pairs are data, not policy, so what matters is
 * that none of them is lost, reordered or mis-quoted on the way out.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import { EMOJI_PATH, EMOJI_SOURCE_PATH, emoji } from './blocks-source.mjs';
import { renderBlocksEmoji } from './blocks-emoji.mjs';

const ts = renderBlocksEmoji();
const entries = ts
  .split('\n')
  .filter((line) => line.startsWith("  '"))
  .map((line) => line.match(/^ {2}'(.*)': '(.*)',$/))
  .map((m) => [m[1], m[2]]);

test('the file opens with a header naming it generated and pointing at the source', () => {
  const head = ts.slice(0, ts.indexOf('*/'));
  for (const line of bannerLines(EMOJI_SOURCE_PATH, EMOJI_PATH)) {
    assert.ok(head.includes(line), `header is missing: ${line}`);
  }
});

test('every pair survives, in the table order', () => {
  assert.deepEqual(entries, emoji);
});

test('the table is sorted by shortcode, which is what the search walks', () => {
  const codes = entries.map(([code]) => code);
  assert.deepEqual([...codes].sort(), codes);
});

test('no shortcode needs escaping it did not get', () => {
  // The gemoji names are `[a-z0-9_+-]`, which the popup's own pattern assumes.
  for (const [code] of entries) assert.match(code, /^[a-z0-9_+-]+$/);
});
