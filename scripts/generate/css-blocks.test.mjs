/**
 * Tests for the stylesheet reader.
 *
 * The generator's tests read the emitted CSS back through this module, so a
 * mis-parse here would pass a wrong value as a right one. Each test below is a
 * shape the reader must refuse rather than guess at.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { parseBlocks } from './css-blocks.mjs';

test('declarations are collected per block, nested blocks by path', () => {
  const css = `
:root {
  color-scheme: dark;
  --bg-0: #000000;
}
@media (prefers-color-scheme: light) {
  :root {
    --bg-0: #FFFFFF;
  }
}
`;
  assert.deepEqual(parseBlocks(css), {
    ':root': { 'color-scheme': 'dark', '--bg-0': '#000000' },
    '@media (prefers-color-scheme: light)': {},
    '@media (prefers-color-scheme: light) > :root': { '--bg-0': '#FFFFFF' },
  });
});

test('comments are dropped, including comments that span lines', () => {
  const css = `
:root {
  /* a note
     over two lines */
  --bg-0: #000000; /* trailing */
}
`;
  assert.deepEqual(parseBlocks(css), { ':root': { '--bg-0': '#000000' } });
});

test('a comment between two values does not join them', () => {
  assert.deepEqual(parseBlocks(':root {\n  --border: 1px/* n */solid;\n}'), {
    ':root': { '--border': '1px solid' },
  });
});

test('two declarations on one line are refused, not silently merged', () => {
  assert.throws(() => parseBlocks(':root {\n  --a: 1px; --b: 2px;\n}'), /unparsed line/);
});

test('a declaration outside any block is refused', () => {
  assert.throws(() => parseBlocks('--a: 1px;'), /declaration outside a block/);
});

test('unbalanced braces are refused', () => {
  assert.throws(() => parseBlocks(':root {\n  --a: 1px;\n'), /unclosed block/);
  assert.throws(() => parseBlocks(':root {\n  --a: 1px;\n}\n}'), /stray closing brace/);
});
