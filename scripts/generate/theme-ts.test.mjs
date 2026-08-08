/**
 * Tests for the typed TypeScript theme generator.
 *
 * These assert relations between the source and the emitted TypeScript — that
 * every declared token reaches a field, that the field carries the same text
 * the stylesheet declares, and that the field names follow the token names.
 * They never restate a token value: `just check` already proves the committed
 * file matches the source, so a literal here would only be a second copy of it.
 *
 * The emitted literals and `themeToVars` are plain JavaScript once their type
 * annotations come off, so the behavioural assertions run the emitted code
 * rather than pattern-matching its text.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import {
  GROUPS,
  RAMPS,
  checkCoverage,
  renderThemeTs,
  themeTokens,
  typeDeclarations,
} from './theme-ts.mjs';
import { formatValue } from './tokens-css.mjs';
import { SOURCE_PATH, inKit, tokenNamed, tokens, valueFor } from '../../packages/tokens/tokens.source.mjs';

const ts = renderThemeTs();

/** Every token the web kit declares — what the theme has to cover. */
const declared = tokens.filter((token) => inKit(token, 'web'));

/** The text the stylesheet declares a token with, in a scheme. */
const authored = (name, scheme) => formatValue(valueFor(tokenNamed(name, 'web'), scheme));

/**
 * One emitted theme literal, evaluated.
 *
 * The literal is valid JavaScript as it stands: the annotation is on the
 * `const`, not inside the braces.
 */
function themeLiteral(name) {
  const open = `export const ${name}: Theme = `;
  const start = ts.indexOf(open);
  assert.ok(start >= 0, `the file emits no ${name}`);
  const end = ts.indexOf('\n};', start);
  assert.ok(end > start, `${name} is not closed`);
  return new Function(`return ${ts.slice(start + open.length, end + 2)}`)();
}

/**
 * The emitted `themeToVars`, made callable.
 *
 * Its body carries one type annotation, on the map it builds. Strip that and
 * the rest is JavaScript, so the test can run the function it emitted rather
 * than assert against its text.
 */
function emittedToVars() {
  const open = 'export function themeToVars(t: Theme): Record<string, string> {';
  const start = ts.indexOf(open);
  assert.ok(start >= 0, 'the file emits no themeToVars');
  const end = ts.indexOf('\n}\n', start);
  assert.ok(end > start, 'themeToVars is not closed');
  const body = ts.slice(start + open.length, end).replaceAll(': Record<string, string>', '');
  return new Function('t', body);
}

const themeToVars = emittedToVars();
const SCHEMES = [
  ['dark', themeLiteral('darkTheme')],
  ['light', themeLiteral('lightTheme')],
];

test('the file opens with a header naming it generated and pointing at the source', () => {
  const head = ts.slice(0, ts.indexOf('*/') + 2);
  for (const line of bannerLines()) assert.ok(head.includes(line), `header is missing: ${line}`);
  assert.ok(head.includes(SOURCE_PATH));
});

test('every token the web kit declares has a field, so none needs the escape hatch', () => {
  const claimed = themeTokens();
  for (const token of declared) {
    assert.ok(claimed.includes(token.name), `the theme has no field for --${token.name}`);
  }
});

test('no token fills two fields', () => {
  const claimed = themeTokens();
  assert.equal(new Set(claimed).size, claimed.length);
});

test('the theme carries no token scoped to another kit', () => {
  const claimed = themeTokens();
  const scoped = tokens.filter((token) => !inKit(token, 'web'));
  assert.ok(scoped.length, 'the source scopes nothing away from the web kit');
  for (const token of scoped) assert.ok(!claimed.includes(token.name), `--${token.name} is not the web kit's`);
});

test('a token the layout forgets fails the generator rather than the theme narrowing', () => {
  const short = themeTokens().filter((name) => name !== 'r-pill');
  assert.throws(() => checkCoverage(short), /no field for --r-pill/);
});

test('a token claimed twice fails the generator', () => {
  assert.throws(() => checkCoverage([...themeTokens(), 'r-pill']), /--r-pill fills more than one field/);
});

test('each built-in theme carries, in every field, the text the stylesheet declares', () => {
  for (const [scheme, theme] of SCHEMES) {
    assert.equal(theme.scheme, scheme);
    for (const ramp of RAMPS) {
      ramp.names.forEach((name, i) => {
        assert.equal(theme[ramp.field][i], authored(name, scheme), `${scheme} ${ramp.field}[${i}]`);
      });
    }
    for (const group of GROUPS) {
      for (const [field, name] of group.fields) {
        assert.equal(theme[group.field][field], authored(name, scheme), `${scheme} ${group.field}.${field}`);
      }
    }
  }
});

test('converting a theme to custom properties emits the whole declared set', () => {
  for (const [scheme, theme] of SCHEMES) {
    const vars = themeToVars(theme);
    assert.deepEqual(
      Object.keys(vars).sort(),
      declared.map((token) => `--${token.name}`).sort(),
      `${scheme} does not emit every declared token`,
    );
    for (const token of declared) {
      assert.equal(vars[`--${token.name}`], authored(token.name, scheme), `${scheme} --${token.name}`);
    }
  }
});

test('the escape hatch is optional, and what it holds outranks the tokens', () => {
  assert.match(ts, /vars\?: Record<`--\$\{string\}`, string>;/);
  const [, dark] = SCHEMES[0];
  const vars = themeToVars({ ...dark, vars: { '--bg-0': 'rebeccapurple', '--per-instance': '3px' } });
  assert.equal(vars['--bg-0'], 'rebeccapurple');
  assert.equal(vars['--per-instance'], '3px');
});

/**
 * The TypeScript field a token name maps to: the name without its group prefix,
 * camel-cased. A property may be quoted, so unlike the Rust kits the leading
 * digit of `--fs-2xl` stays where the source put it.
 */
const fieldFor = (name, prefix) => name.slice(prefix.length).replace(/-(.)/g, (_, c) => c.toUpperCase());

test('a field named after its token uses the token name, not a synonym', () => {
  const named = GROUPS.filter((group) => group.prefix !== undefined);
  assert.ok(named.length, 'no group names its fields after its tokens');
  for (const group of named) {
    for (const [field, name] of group.fields) {
      assert.ok(name.startsWith(group.prefix), `--${name} is not in the ${group.prefix}* group`);
      assert.equal(field, fieldFor(name, group.prefix), `${group.field}.${field} renames --${name}`);
    }
  }
});

test('every group is declared as a type, and the semantic tones share one', () => {
  for (const group of GROUPS) {
    assert.match(ts, new RegExp(`export interface ${group.type} \\{`), `${group.type} is not declared`);
    assert.match(ts, new RegExp(`\\n  ${group.field}: ${group.type};`), `Theme has no ${group.field}`);
  }
  const tones = GROUPS.filter((group) => group.type === 'SemanticTriple');
  assert.ok(tones.length > 1, 'the tones do not share a type');
  assert.equal(ts.match(/export interface SemanticTriple \{/g).length, 1);
});

test('one type is declared once, from the first group that claims it', () => {
  const claimants = typeDeclarations(GROUPS).map((group) => group.type);
  assert.equal(new Set(claimants).size, claimants.length);
  assert.equal(claimants.length, new Set(GROUPS.map((group) => group.type)).size);
});

test('two groups claiming one type with different fields is refused, not emitted twice', () => {
  const imposter = {
    field: 'imposter',
    type: 'SemanticTriple',
    typeDoc: ['A tone of another shape.'],
    doc: 'An imposter.',
    fields: [['only', 'accent', 'Not what the tones declare.']],
  };
  assert.throws(() => typeDeclarations([...GROUPS, imposter]), /both declare SemanticTriple/);
});
