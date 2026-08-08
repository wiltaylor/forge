/**
 * Tests for the CSS generator.
 *
 * These assert relations between the source and the emitted stylesheet — which
 * blocks declare which tokens, and that the repeated blocks agree. They never
 * restate a token value: `just check` already proves the committed stylesheet
 * matches the source, so a literal here would only be a second copy of it.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import { parseBlocks } from './css-blocks.mjs';
import { formatValue, renderTokensCss } from './tokens-css.mjs';
import {
  SOURCE_PATH,
  groups,
  isSchemeToken,
  tokens,
  tokensFor,
  valueFor,
} from '../../packages/tokens/tokens.source.mjs';

const css = renderTokensCss();
const blocks = parseBlocks(css);

const ROOT = ':root';
const MEDIA_LIGHT = '@media (prefers-color-scheme: light) > :root:not([data-theme="dark"])';
const ATTR_LIGHT = '[data-theme="light"]';
const ATTR_DARK = '[data-theme="dark"]';

const names = (block) => Object.keys(blocks[block]).filter((n) => n.startsWith('--'));

test('the file opens with a header naming it generated and pointing at the source', () => {
  const head = css.slice(0, css.indexOf('*/'));
  for (const line of bannerLines()) assert.ok(head.includes(line), `header is missing: ${line}`);
  assert.ok(head.includes(SOURCE_PATH));
});

test('every block the stylesheet needs is emitted', () => {
  assert.deepEqual(Object.keys(blocks).sort(), [
    ATTR_DARK,
    ATTR_LIGHT,
    MEDIA_LIGHT,
    ROOT,
    '@media (prefers-color-scheme: light)',
  ].sort());
});

test(':root declares the whole web token set and nothing else', () => {
  assert.deepEqual(names(ROOT), tokensFor('web').map((t) => `--${t.name}`));
});

test('the override blocks restate exactly the per-scheme tokens', () => {
  const scheme = tokensFor('web').filter(isSchemeToken).map((t) => `--${t.name}`);
  for (const block of [MEDIA_LIGHT, ATTR_LIGHT, ATTR_DARK]) assert.deepEqual(names(block), scheme);
});

test('a token scoped to another kit gets no custom property', () => {
  const scoped = tokens.filter((t) => t.only && !t.only.includes('web'));
  assert.ok(scoped.length, 'the source scopes nothing away from the web');
  for (const token of scoped) assert.doesNotMatch(css, new RegExp(`--${token.name}\\b`), token.name);
});

test('each block declares its own color-scheme', () => {
  assert.equal(blocks[ROOT]['color-scheme'], 'dark');
  assert.equal(blocks[ATTR_DARK]['color-scheme'], 'dark');
  assert.equal(blocks[MEDIA_LIGHT]['color-scheme'], 'light');
  assert.equal(blocks[ATTR_LIGHT]['color-scheme'], 'light');
});

test('the light scheme is stated once — both light blocks agree', () => {
  assert.deepEqual(blocks[ATTR_LIGHT], blocks[MEDIA_LIGHT]);
});

test('every declaration carries the value the source authored for that scheme', () => {
  for (const [block, scheme] of [
    [ROOT, 'dark'],
    [ATTR_DARK, 'dark'],
    [ATTR_LIGHT, 'light'],
    [MEDIA_LIGHT, 'light'],
  ]) {
    for (const token of tokens) {
      const declared = blocks[block][`--${token.name}`];
      if (declared === undefined) continue;
      assert.equal(declared, formatValue(valueFor(token, scheme)), `--${token.name} in ${block}`);
    }
  }
});

test('a tint declares its base colour at the authored alpha', () => {
  const tints = tokens.filter((t) => isSchemeToken(t) && t.dark.alpha !== undefined);
  assert.ok(tints.length, 'the source has no tints to check');
  for (const token of tints) {
    for (const scheme of ['dark', 'light']) {
      const value = valueFor(token, scheme);
      const opaque = formatValue({ oklch: value.oklch });
      assert.equal(
        formatValue(value),
        `${opaque.slice(0, -1)} / ${value.alpha.toFixed(2)})`,
        `--${token.name} (${scheme})`,
      );
    }
  }
});

test('every tint names the surface it flattens over, and that surface is a token', () => {
  const declared = new Set(tokens.map((t) => t.name));
  for (const token of tokens.filter(isSchemeToken)) {
    for (const scheme of ['dark', 'light']) {
      const value = valueFor(token, scheme);
      if (value.alpha === undefined) continue;
      assert.ok(declared.has(value.over), `--${token.name} flattens over unknown --${value.over}`);
    }
  }
});

test('a token is authored either once or per scheme, never both and never neither', () => {
  for (const token of tokens) {
    const perScheme = token.dark !== undefined && token.light !== undefined;
    const shared = token.value !== undefined;
    assert.ok(perScheme !== shared, `--${token.name} is authored ambiguously`);
  }
});

test('token names are unique', () => {
  assert.equal(new Set(tokens.map((t) => t.name)).size, tokens.length);
});

test('a group is either a comment note or a run of tokens', () => {
  for (const group of groups) assert.ok(group.comment || group.tokens?.length, 'empty group');
});
