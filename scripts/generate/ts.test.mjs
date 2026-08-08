/**
 * Tests for the pieces the TypeScript generators share.
 *
 * Both generated TypeScript files spell their strings, quote their keys, wrap
 * their literals and write their JSDoc through this module, so these assert the
 * decisions it makes rather than any one file's output.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { docComment, plainKey, propertyKey, quote } from './ts.mjs';

test('a string is written in whichever quote it needs no escape in', () => {
  assert.equal(quote('plain'), "'plain'");
  assert.equal(quote("it's"), '"it\'s"');
  assert.equal(quote('say "hi"'), '\'say "hi"\'');
});

test('a string holding both quotes escapes the one it is written in', () => {
  assert.equal(quote('it\'s "so"'), "'it\\'s \"so\"'");
});

test('a backslash is escaped whichever quote wins', () => {
  assert.equal(quote('a\\b'), "'a\\\\b'");
});

test('a key is written bare when it stands as one, and quoted when it does not', () => {
  assert.equal(propertyKey('base'), 'base');
  assert.equal(propertyKey('$id'), '$id');
  assert.equal(propertyKey('4'), '4');
  assert.equal(propertyKey('2xl'), "'2xl'");
  assert.equal(propertyKey('wrap-columns'), "'wrap-columns'");
});

test('a leading digit stands only where the whole key is digits', () => {
  assert.ok(plainKey('16'));
  assert.ok(!plainKey('16px'));
});

test('a doc comment of one line stays on one line', () => {
  assert.deepEqual(docComment(['only'], '  '), ['  /** only */']);
  assert.deepEqual(docComment([], '  '), []);
  assert.deepEqual(docComment(['one', 'two']), ['/** one', '    two */']);
});

test('a paragraph break inside a doc comment carries no trailing space', () => {
  assert.deepEqual(docComment(['one', '', 'two'], '  '), ['  /** one', '', '      two */']);
});
