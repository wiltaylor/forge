#!/usr/bin/env node
/**
 * The `test` task of a package that has no tests yet.
 *
 *   "test": "node ../../scripts/no-tests-yet.mjs"
 *
 * A package without a `test` script is silently absent from the aggregate
 * `turbo test` run. This script is the explicit alternative: the run reports
 * the package as covered-by-declaration instead of not at all.
 *
 * It also guards the declaration. If a test file exists in the package, the
 * declaration is a lie — the file would sit there and never run — so the
 * script fails and points at docs/web-testing.md, which describes the real
 * test setup that must replace it.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

/** Directories that hold no authored source (same set as token-scan.mjs, plus dist-remote). */
const SKIP = new Set(['.git', 'node_modules', 'dist', 'dist-remote', 'target', 'vendor', '.turbo', '.venv']);

/** What vitest collects by default: `**\/*.{test,spec}.?(c|m)[jt]s?(x)`. */
const TEST_FILE = /\.(test|spec)\.[cm]?[jt]sx?$/;

/** Every test file under `dir`, relative to it and in walk order. */
export function findTestFiles(dir, root = dir) {
  const found = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!SKIP.has(entry.name)) found.push(...findTestFiles(path, root));
    } else if (TEST_FILE.test(entry.name)) {
      found.push(relative(root, path));
    }
  }
  return found;
}

function main() {
  const dir = process.cwd();
  const name = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8')).name;
  const files = findTestFiles(dir);
  if (files.length > 0) {
    console.error(`${name} declares "no tests yet", but test files exist:`);
    for (const file of files) console.error(`  ${file}`);
    console.error('Replace the declaration with a real test script — see docs/web-testing.md.');
    process.exit(1);
  }
  console.log(`${name}: no tests yet — declared so the aggregate run reports it, not skips it.`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
