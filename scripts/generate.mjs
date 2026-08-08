#!/usr/bin/env node
/**
 * Regenerate every artifact derived from the token source, or check that the
 * committed ones are up to date.
 *
 *   node scripts/generate.mjs           write every artifact  (`just generate`)
 *   node scripts/generate.mjs --check   fail if any differs   (`just check`)
 *
 * The check compares contents rather than asking git, so it fails only for a
 * file that the source no longer produces — an unrelated edit elsewhere in the
 * tree, staged or not, is none of its business.
 *
 * Adding an output means adding one entry to `ARTIFACTS`.
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { renderTokensCss } from './generate/tokens-css.mjs';

const REPO = dirname(dirname(fileURLToPath(import.meta.url)));

/** Every generated file: repo-relative path, and the function that emits its text. */
const ARTIFACTS = [{ path: 'packages/tokens/css/tokens.css', render: renderTokensCss }];

async function currentText(path) {
  try {
    return await readFile(join(REPO, path), 'utf8');
  } catch (err) {
    if (err.code === 'ENOENT') return null;
    throw err;
  }
}

async function main(check) {
  const stale = [];
  for (const artifact of ARTIFACTS) {
    const wanted = artifact.render();
    if ((await currentText(artifact.path)) === wanted) continue;
    stale.push(artifact.path);
    if (!check) await writeFile(join(REPO, artifact.path), wanted);
  }

  if (!stale.length) {
    console.log(`${ARTIFACTS.length} generated file(s) up to date`);
    return 0;
  }
  if (!check) {
    for (const path of stale) console.log(`wrote ${path}`);
    return 0;
  }
  console.error('These generated files do not match the token source:\n');
  for (const path of stale) console.error(`  ${path}`);
  console.error('\nRun `just generate` and commit the result.');
  return 1;
}

process.exitCode = await main(process.argv.includes('--check'));
