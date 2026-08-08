/**
 * Tests for the two Rust palette generators.
 *
 * These assert the derivation rules — what a tint becomes in a kit that has no
 * alpha channel, and in one that has — by recomputing them from the source and
 * comparing against the emitted text. They never restate a token value.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import { alphaByte, flatten, toRgb } from './oklch.mjs';
import { alphaConstants, hexLiteral, renderEguiPalette, renderTuiPalette } from './rust-palette.mjs';
import { SOURCE_PATH, tokenNamed, tokens, valueFor } from '../../packages/tokens/tokens.source.mjs';

const tui = renderTuiPalette();
const egui = renderEguiPalette();

/** The text of one scheme's `Theme` literal, between its opening line and the next. */
function schemeBlock(source, opener) {
  const start = source.indexOf(opener);
  assert.notEqual(start, -1, `no block opens with ${opener}`);
  const rest = source.slice(start + opener.length);
  const end = rest.search(/\n(pub |$)/);
  return end === -1 ? rest : rest.slice(0, end);
}

const BLOCKS = [
  { kit: 'tui', scheme: 'dark', text: schemeBlock(tui, 'pub const DARK: Theme = Theme {') },
  { kit: 'tui', scheme: 'light', text: schemeBlock(tui, 'pub const LIGHT: Theme = Theme {') },
  { kit: 'egui', scheme: 'dark', text: schemeBlock(egui, 'pub fn dark() -> Theme {') },
  { kit: 'egui', scheme: 'light', text: schemeBlock(egui, 'pub fn light() -> Theme {') },
];

const block = (kit, scheme) => BLOCKS.find((b) => b.kit === kit && b.scheme === scheme).text;

/** The expression a named struct field is assigned, without its comma or comment. */
function field(text, name) {
  const line = fieldLine(text, name);
  return line.match(new RegExp(`^\\s*${name}: (.+?),(?: +//.*)?$`))[1];
}

/** The whole emitted line a named struct field sits on, comment included. */
function fieldLine(text, name) {
  const match = text.match(new RegExp(`^\\s*${name}: .+?,(?: +//.*)?$`, 'm'));
  assert.ok(match, `no field "${name}" in the block`);
  return match[0];
}

/** The body of one nested struct — `accent: Accent { … }` — up to its closing brace. */
function nested(text, name) {
  const start = text.indexOf(`${name}: `);
  assert.notEqual(start, -1, `no struct "${name}" in the block`);
  const body = text.slice(start);
  const end = body.search(/^\s*\},$/m);
  return end === -1 ? body : body.slice(0, end);
}

/** The nested struct a `*-bg` token's tint lands in: `accent-bg` is `accent.bg`. */
const tintStruct = (token) => token.name.replace(/-bg$/, '');

/** Every tint in the source, as `{ token, scheme, value }`. */
const TINTS = tokens.flatMap((token) =>
  ['dark', 'light']
    .map((scheme) => ({ token, scheme, value: valueFor(token, scheme) }))
    .filter(({ value }) => value !== undefined && value.alpha !== undefined),
);

test('the source has tints for both schemes, or the rules below prove nothing', () => {
  assert.ok(TINTS.length >= 10, `only ${TINTS.length} tints found`);
});

for (const [name, text] of [
  ['forge-tui', tui],
  ['forge-egui', egui],
]) {
  test(`${name}'s palette opens with a header naming it generated and pointing at the source`, () => {
    const head = text.slice(0, text.indexOf('\n//!\n'));
    for (const line of bannerLines()) assert.ok(head.includes(line), `header is missing: ${line}`);
    assert.ok(head.includes(SOURCE_PATH));
  });

  test(`${name}'s palette declares both schemes`, () => {
    assert.match(text, /forge-dark/);
    assert.match(text, /forge-light/);
    assert.match(text, /Scheme::Dark/);
    assert.match(text, /Scheme::Light/);
  });
}

/** Every colour literal in a block except the tints, which are the `bg:` fields. */
const opaqueColours = (text) =>
  text
    .split('\n')
    .filter((line) => !/^\s*bg: /.test(line))
    .join('\n')
    .match(/0x[0-9A-F]{6}/g);

test('the two kits state the same colours wherever a tint is not involved', () => {
  for (const scheme of ['dark', 'light']) {
    assert.deepEqual(
      opaqueColours(block('tui', scheme)),
      opaqueColours(block('egui', scheme)),
      `the kits disagree about the ${scheme} scheme`,
    );
  }
});

test('forge-tui has no alpha anywhere — a terminal cannot paint it', () => {
  assert.doesNotMatch(tui, /with_alpha/);
});

test("forge-tui's tints are flattened over the surface the source names", () => {
  for (const { token, scheme, value } of TINTS) {
    const surface = valueFor(tokenNamed(value.over), scheme);
    const composited = flatten(toRgb(value), toRgb(surface), value.alpha);
    assert.equal(
      field(nested(block('tui', scheme), tintStruct(token)), 'bg'),
      `rgb(${hexLiteral(composited)})`,
      `--${token.name} (${scheme}) is not its base over --${value.over}`,
    );
  }
});

test("forge-egui's tints carry their own colour with the alpha as a byte", () => {
  for (const { token, scheme, value } of TINTS) {
    const [, literal, constant] = field(nested(block('egui', scheme), tintStruct(token)), 'bg').match(
      /^with_alpha\((rgb\(0x[0-9A-F]{6}\)), (A\d+)\)$/,
    );
    assert.equal(literal, `rgb(${hexLiteral(toRgb(value))})`, `--${token.name} (${scheme})`);
    assert.match(
      egui,
      new RegExp(`^const ${constant}: u8 = ${alphaByte(value.alpha)};$`, 'm'),
      `--${token.name} (${scheme}) names ${constant}, which is not declared as its alpha`,
    );
  }
});

test('forge-egui declares an alpha constant for every alpha it uses, and no others', () => {
  const declared = [...egui.matchAll(/^const (A\d+): u8 =/gm)].map((m) => m[1]);
  const used = [...new Set([...egui.matchAll(/with_alpha\(.+, (A\d+)\)/g)].map((m) => m[1]))];
  assert.deepEqual(declared.sort(), used.sort());
});

test('two alphas that want one constant name are refused, not emitted twice', () => {
  assert.deepEqual(alphaConstants([0.14, 0.2]), [
    ['A14', 0.14],
    ['A20', 0.2],
  ]);
  // Both round to 15%, so both would emit `const A15` — with different values.
  assert.throws(() => alphaConstants([0.151, 0.153]), /both want the constant A15/);
});

test('a byte triple formats as the literal the kits take', () => {
  assert.equal(hexLiteral([0x0a, 0x1b, 0x2c]), '0x0A1B2C');
  assert.equal(hexLiteral([0, 0, 0]), '0x000000', 'each channel is padded to two digits');
});

test('every tint states its authored expression, and forge-tui names the surface too', () => {
  for (const { token, scheme, value } of TINTS) {
    const alpha = value.alpha.toFixed(2);
    const expression = new RegExp(`// oklch\\([\\d. ]+ / ${alpha}\\)`);
    for (const kit of ['tui', 'egui']) {
      const line = fieldLine(nested(block(kit, scheme), tintStruct(token)), 'bg');
      assert.match(line, expression, `--${token.name} (${scheme}) in forge-${kit}`);
      if (kit === 'tui') assert.ok(line.endsWith(` over ${value.over}`), line);
    }
  }
});
