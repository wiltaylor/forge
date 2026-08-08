#!/usr/bin/env node
/**
 * Fail when anything references a design token the source never declared.
 *
 *   node scripts/token-scan.mjs      (`just token-scan`, part of `just check`)
 *
 * This walks the tree, reads every stylesheet and every TypeScript file that
 * can hold a token reference, and judges each reference against the names
 * `packages/tokens/tokens.source.mjs` declares for the web kit — the same set
 * the generator emits into `packages/tokens/css/tokens.css`. The rules, the
 * allowlist of per-instance properties and the fallback exemptions all live in
 * `scripts/token-scan/scan.mjs`.
 *
 * Node only, like the generator, so `just check` needs no other toolchain.
 */
import { readdir, readFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import { inKit, tokens } from '../packages/tokens/tokens.source.mjs';
import { isScanned, violations } from './token-scan/scan.mjs';

const REPO = dirname(dirname(fileURLToPath(import.meta.url)));

/** Directories that hold no authored source. */
const SKIP = new Set(['.git', 'node_modules', 'dist', 'target', 'vendor', '.turbo', '.venv']);

/** Every scannable file under `dir`, repo-relative and sorted. */
async function walk(dir) {
  const found = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP.has(entry.name)) continue;
      found.push(...(await walk(path)));
    } else if (entry.isFile() && isScanned(entry.name)) {
      found.push(relative(REPO, path));
    }
  }
  return found.sort();
}

async function main() {
  const declared = new Set(tokens.filter((t) => inKit(t, 'web')).map((t) => `--${t.name}`));
  const paths = await walk(REPO);
  const files = await Promise.all(
    paths.map(async (path) => ({ path, text: await readFile(join(REPO, path), 'utf8') })),
  );

  const found = violations(files, declared);
  if (!found.length) {
    console.log(`${files.length} file(s) scanned, ${declared.size} declared token(s), no violations`);
    return 0;
  }

  console.error('These token references do not match the token source:\n');
  for (const v of found) {
    const at = v.line ? `${v.path}:${v.line}` : v.path;
    console.error(`  ${at}  ${v.name} — ${v.problem}`);
  }
  console.error(`\nDeclare the token in packages/tokens/tokens.source.mjs and run \`just generate\`,`);
  console.error('or reference a name that exists.');
  return 1;
}

process.exitCode = await main();
