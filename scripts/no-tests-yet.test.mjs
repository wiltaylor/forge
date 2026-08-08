/**
 * Tests for the no-tests-yet declaration script.
 *
 * The script is the `test` task of every package that has no tests, so the
 * aggregate `turbo test` run reports the package instead of skipping it. The
 * property worth proving is the guard: the moment a real test file appears in
 * such a package, the declaration must fail loudly — otherwise the file would
 * exist and never run, which is the silent skip the script exists to end.
 */
import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { findTestFiles } from './no-tests-yet.mjs';

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), 'no-tests-yet.mjs');

/** A throwaway package directory with the given files (paths relative to it). */
function pkg(files) {
  const dir = mkdtempSync(join(tmpdir(), 'no-tests-yet-'));
  writeFileSync(join(dir, 'package.json'), JSON.stringify({ name: '@forge/fixture' }));
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(join(dir, dirname(path)), { recursive: true });
    writeFileSync(join(dir, path), content);
  }
  return dir;
}

function run(dir) {
  return spawnSync(process.execPath, [SCRIPT], { cwd: dir, encoding: 'utf8' });
}

test('a package without test files declares itself and passes', () => {
  const dir = pkg({ 'src/index.ts': 'export {};' });
  const r = run(dir);
  assert.equal(r.status, 0, r.stderr);
  assert.match(r.stdout, /@forge\/fixture/);
  assert.match(r.stdout, /no tests yet/);
});

test('a test file anywhere in the package fails the declaration', () => {
  const dir = pkg({ 'src/deep/thing.test.ts': 'export {};' });
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /thing\.test\.ts/);
  assert.match(r.stderr, /docs\/web-testing\.md/, 'points at the adoption doc');
});

test('every extension vitest would collect trips the guard', () => {
  for (const name of ['a.test.ts', 'a.test.tsx', 'a.spec.js', 'a.test.mjs', 'a.spec.cts']) {
    const dir = pkg({ [join('src', name)]: '' });
    assert.deepEqual(
      findTestFiles(dir).map((f) => f.split('/').pop()),
      [name],
      name,
    );
  }
});

test('build output and dependencies do not trip the guard', () => {
  const dir = pkg({
    'node_modules/dep/index.test.js': '',
    'dist/index.test.js': '',
    '.turbo/index.test.js': '',
    'src-tauri/target/debug/build/pkg/out/index.test.js': '',
  });
  assert.deepEqual(findTestFiles(dir), []);
});
