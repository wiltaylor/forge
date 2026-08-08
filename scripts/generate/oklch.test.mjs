/**
 * Tests for the colour conversion.
 *
 * These assert the rules the module states — chroma gives way at the gamut
 * boundary, a composite is a weighted mix, alpha quantises to a byte — rather
 * than the sRGB value of any particular token. `just check` already proves the
 * committed palettes match the source, so a literal here would only be another
 * copy of one.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { alphaByte, flatten, hexToRgb, oklchToRgb, toRgb } from './oklch.mjs';
import { isSchemeToken, tokens, valueFor } from '../../packages/tokens/tokens.source.mjs';

/** Well outside sRGB at every hue: no display can show this much chroma. */
const IMPOSSIBLE_CHROMA = 0.4;

const isByte = (v) => Number.isInteger(v) && v >= 0 && v <= 255;

/** A colour no scheme in the source authors, so no assertion below copies a token. */
const NOT_A_TOKEN = '#0A1B2C';

test('a hex literal parses to its three bytes', () => {
  assert.deepEqual(hexToRgb(NOT_A_TOKEN), [0x0a, 0x1b, 0x2c]);
  assert.deepEqual(hexToRgb('0A1B2C'), [0x0a, 0x1b, 0x2c], 'the leading # is optional');
  assert.throws(() => hexToRgb('#abc'));
});

test('an achromatic colour converts to a grey', () => {
  for (const lightness of [0, 0.25, 0.5, 0.75, 1]) {
    const [r, g, b] = oklchToRgb([lightness, 0, 0]);
    assert.equal(r, g, `L=${lightness}`);
    assert.equal(g, b, `L=${lightness}`);
  }
  assert.deepEqual(oklchToRgb([0, 0, 0]), [0, 0, 0]);
  assert.deepEqual(oklchToRgb([1, 0, 0]), [255, 255, 255]);
});

test('every conversion lands on three bytes', () => {
  for (let hue = 0; hue < 360; hue += 15) {
    for (const lightness of [0.1, 0.4, 0.7, 0.95]) {
      const rgb = oklchToRgb([lightness, IMPOSSIBLE_CHROMA, hue]);
      assert.ok(rgb.every(isByte), `oklch(${lightness} ${IMPOSSIBLE_CHROMA} ${hue}) -> ${rgb}`);
    }
  }
});

test('chroma gives way at the gamut boundary — asking for more changes nothing', () => {
  for (let hue = 0; hue < 360; hue += 15) {
    const fitted = oklchToRgb([0.6, IMPOSSIBLE_CHROMA, hue]);
    assert.deepEqual(
      oklchToRgb([0.6, IMPOSSIBLE_CHROMA * 2, hue]),
      fitted,
      `hue ${hue} kept moving past the gamut boundary`,
    );
  }
});

test('a colour inside the gamut is converted, not fitted', () => {
  // Doubling a small chroma stays reachable, so the result must move.
  assert.notDeepEqual(oklchToRgb([0.6, 0.02, 250]), oklchToRgb([0.6, 0.04, 250]));
});

test('a composite is a weighted mix of the two colours', () => {
  const white = [255, 255, 255];
  const black = [0, 0, 0];
  assert.deepEqual(flatten(white, black, 0), black, 'alpha 0 is the surface');
  assert.deepEqual(flatten(white, black, 1), white, 'alpha 1 is the tint');
  assert.deepEqual(flatten(white, black, 0.5), [128, 128, 128]);
  // A tint of a colour over itself is that colour, whatever the alpha.
  const colour = [100, 200, 50];
  assert.deepEqual(flatten(colour, colour, 0.14), colour);
});

test('alpha quantises to a byte, rounding half up', () => {
  assert.equal(alphaByte(0), 0);
  assert.equal(alphaByte(1), 255);
  assert.equal(alphaByte(0.5), 128, '127.5 rounds up');
});

test('every colour the source authors converts', () => {
  for (const token of tokens) {
    for (const scheme of ['dark', 'light']) {
      const value = valueFor(token, scheme);
      if (value === undefined || value.raw !== undefined) continue;
      const rgb = toRgb(value);
      assert.ok(rgb.every(isByte), `--${token.name} (${scheme}) -> ${rgb}`);
    }
  }
});

test('a tint converts to the same colour as the opaque value it is a tint of', () => {
  const tints = tokens.filter((t) => isSchemeToken(t) && t.dark.alpha !== undefined);
  assert.ok(tints.length, 'the source has no tints to check');
  for (const token of tints) {
    for (const scheme of ['dark', 'light']) {
      const value = valueFor(token, scheme);
      assert.deepEqual(toRgb(value), oklchToRgb(value.oklch), `--${token.name} (${scheme})`);
    }
  }
});

test('a value that is not a colour is refused rather than guessed at', () => {
  assert.throws(() => toRgb({ raw: '4px' }));
  assert.throws(() => toRgb({}));
});
