/**
 * Tests for the skill CSS bundler.
 *
 * The one relation worth asserting is order: a bundle concatenated in the
 * wrong order is present, plausible and subtly wrong in its cascade. So these
 * check the manifest against the order facts the source headers document, and
 * that the emitted bundles lay their sections out in manifest order. They
 * never restate a bundle's contents: `just check` already proves the committed
 * bundles match the package stylesheets.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { bannerLines } from './banner.mjs';
import { BUNDLES, renderBundle, sectionMarker } from './skill-css.mjs';

const REPO = dirname(dirname(dirname(fileURLToPath(import.meta.url))));

/** The first comment block of a source stylesheet — where it documents its order. */
const header = (source) => {
  const css = readFileSync(join(REPO, source), 'utf8');
  return css.slice(0, css.indexOf('*/'));
};

/** Manifest position of a source within its bundle. */
const position = (bundle, source) => {
  const at = bundle.sources.indexOf(source);
  assert.notEqual(at, -1, `${bundle.path} does not bundle ${source}`);
  return at;
};

const [colorsBundle, consoleBundle, chatBundle] = BUNDLES;

test('fonts.css documents "import FIRST" and the manifest puts it first', () => {
  assert.match(header('packages/tokens/css/fonts.css'), /import FIRST/);
  assert.equal(position(colorsBundle, 'packages/tokens/css/fonts.css'), 0);
});

test('base.css documents "Import after tokens.css" and the manifest obeys', () => {
  assert.match(header('packages/tokens/css/base.css'), /Import after tokens\.css/);
  assert.ok(
    position(colorsBundle, 'packages/tokens/css/base.css') >
      position(colorsBundle, 'packages/tokens/css/tokens.css'),
  );
});

test('the extracted stylesheets follow the console core they were cut from', () => {
  const core = position(consoleBundle, 'packages/ui/styles/console.css');
  assert.equal(core, 0);
  for (const pkg of ['charts', 'graph', 'code']) {
    const source = `packages/${pkg}/styles/${pkg}.css`;
    assert.match(header(source), /Extracted from console\.css/);
    assert.ok(position(consoleBundle, source) > core);
  }
});

test('chat.css documents its place after the ui styles, and its bundle says so', () => {
  assert.match(header('packages/chat/styles/chat.css'), /Import after @forge\/ui\/styles\.css/);
  assert.match(chatBundle.note, /after console\.css/);
});

const rendered = BUNDLES.map((bundle) => [bundle, renderBundle(bundle)]);

test('each bundle opens with the generated banner naming every source', () => {
  for (const [bundle, css] of rendered) {
    const head = css.slice(0, css.indexOf('*/'));
    for (const line of bannerLines(bundle.sources)) {
      assert.ok(head.includes(line), `${bundle.path} banner is missing: ${line}`);
    }
  }
});

test("each bundle's sections appear in the documented order", () => {
  for (const [bundle, css] of rendered) {
    let last = -1;
    for (const source of bundle.sources) {
      const at = css.indexOf(sectionMarker(source));
      assert.ok(at > last, `${bundle.path}: ${source} is out of order or missing`);
      last = at;
    }
  }
});
