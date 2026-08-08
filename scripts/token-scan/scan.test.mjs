import assert from 'node:assert/strict';
import test from 'node:test';

import {
  FALLBACK_EXEMPT,
  PER_INSTANCE,
  isScanned,
  references,
  stripComments,
  violations,
} from './scan.mjs';

/** The declared set the tree really has is irrelevant here; these are enough. */
const DECLARED = new Set(['--border', '--r-md', '--dur-1']);

const css = (path, text) => ({ path, text });

test('reads a plain var() reference', () => {
  assert.deepEqual(references('a { color: var(--border); }'), [
    { name: '--border', fallback: false, line: 1 },
  ]);
});

test('reports the line a reference sits on', () => {
  const text = 'a {\n\n  color: var(--border);\n}\n';
  assert.equal(references(text)[0].line, 3);
});

test('marks a reference that carries a fallback', () => {
  const [ref] = references('a { border-radius: var(--r-md, 6px); }');
  assert.equal(ref.fallback, true);
});

test('ignores a name written inside a comment', () => {
  assert.deepEqual(references('/* every value is a var(--token) */\na { top: 0; }'), []);
});

test('a comment does not shift the lines after it', () => {
  const text = '/* one\n   two */\na { color: var(--border); }';
  assert.equal(references(text)[0].line, 3);
});

test('a name in a string is a reference, not prose', () => {
  const [ref] = references("theme({ '&': { color: 'var(--border)' } });", { lineComments: true });
  assert.deepEqual(ref, { name: '--border', fallback: false, line: 1 });
});

test('reads a quoted property name written from JavaScript', () => {
  const [ref] = references("<div style={{ '--fbk-cols': n }} />", { lineComments: true });
  assert.deepEqual(ref, { name: '--fbk-cols', fallback: false, line: 1 });
});

test('a literal after a quoted name is a fallback', () => {
  const [ref] = references("v('--accent', '#5A8FDB')", { lineComments: true });
  assert.equal(ref.fallback, true);
});

test('a second property name is another reference, not a fallback', () => {
  assert.deepEqual(references("mix('--danger', '--accent')", { lineComments: true }), [
    { name: '--danger', fallback: false, line: 1 },
    { name: '--accent', fallback: false, line: 1 },
  ]);
});

test('`---` in Markdown is not a property name', () => {
  assert.deepEqual(references("split('---')", { lineComments: true }), []);
});

test('a line comment hides a reference only where // opens one', () => {
  const text = '// color: var(--border)\n';
  assert.deepEqual(references(text, { lineComments: true }), []);
  assert.equal(references(text).length, 1);
});

test('stripComments leaves string contents alone', () => {
  assert.equal(stripComments("a: '/* not a comment */';"), "a: '/* not a comment */';");
});

test('stripComments does not read a URL as a line comment', () => {
  const text = "const u = 'https://example.test/x'; // gone\n";
  assert.equal(stripComments(text, { lineComments: true }).trimEnd(), "const u = 'https://example.test/x';");
});

test('an undeclared name is a violation', () => {
  const found = violations([css('a.css', 'a { color: var(--nope); }')], DECLARED);
  assert.equal(found[0].name, '--nope');
  assert.match(found[0].problem, /no token of this name/);
  assert.equal(found[0].path, 'a.css');
  assert.equal(found[0].line, 1);
});

test('a declared name is not a violation', () => {
  const found = violations([css('a.css', 'a { color: var(--border); }')], DECLARED);
  assert.deepEqual(problemsFor(found, 'a.css'), []);
});

test('a fallback on a declared token is a violation', () => {
  const found = violations([css('a.css', 'a { border-radius: var(--r-md, 6px); }')], DECLARED);
  assert.match(problemsFor(found, 'a.css')[0], /needs no fallback/);
});

test('an allowlisted per-instance property passes, fallback and all', () => {
  assert.deepEqual(problemsFor(violations(allowlistUse(), DECLARED), 'use.css'), []);
});

test('an exempt file keeps its fallbacks', () => {
  const [exempt] = FALLBACK_EXEMPT;
  const files = allowlistUse().concat(css(exempt.file, "v('--border', '#000000')"));
  assert.deepEqual(violations(files, DECLARED), []);
});

test('the same fallback outside the exempt file fails', () => {
  const files = allowlistUse().concat(css('other.ts', "v('--border', '#000000')"));
  assert.match(problemsFor(violations(files, DECLARED), 'other.ts')[0], /needs no fallback/);
});

test('an allowlist entry nothing references is itself a violation', () => {
  const found = violations([css(FALLBACK_EXEMPT[0].file, "v('--border', '#000')")], DECLARED);
  const names = found.map((v) => v.name);
  for (const entry of PER_INSTANCE) assert.ok(names.includes(entry.name), entry.name);
});

test('an exemption nothing needs is itself a violation', () => {
  const found = violations(allowlistUse(), DECLARED);
  assert.deepEqual(
    found.map((v) => v.name),
    FALLBACK_EXEMPT.map((entry) => entry.file),
  );
});

test('only stylesheets and TypeScript are scanned', () => {
  assert.ok(isScanned('a/b.css'));
  assert.ok(isScanned('a/b.ts'));
  assert.ok(isScanned('a/b.tsx'));
  assert.ok(!isScanned('a/b.rs'));
  assert.ok(!isScanned('README'));
});

/** One file that references every allowlisted property, so the rot check rests. */
function allowlistUse() {
  return [css('use.css', PER_INSTANCE.map((e) => `a { top: var(${e.name}, 0); }`).join('\n'))];
}

const problemsFor = (found, path) => found.filter((v) => v.path === path).map((v) => v.problem);
