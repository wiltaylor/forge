#!/usr/bin/env node
/**
 * Regenerate every artifact derived from an authored source, or check that the
 * committed ones are up to date.
 *
 *   node scripts/generate.mjs           write every artifact  (`just generate`)
 *   node scripts/generate.mjs --check   fail if any differs   (`just check`)
 *
 * The check compares file contents. It does not ask git. Thus it fails only for
 * a file that no longer matches its source. It ignores every other edit in the
 * tree, staged or not.
 *
 * Two sources feed it. The design tokens are authored in JavaScript and read
 * directly. The block kind registry is Rust, so `just generate-blocks` dumps it
 * to `contract/*.json` first and these generators read that — which keeps this
 * script, and `just check` with it, a Node-only job that installing the web kit
 * never needs a Rust toolchain to reproduce. A dump states a digest of its Rust
 * source, so a stale one fails here rather than quietly generating from it.
 *
 * To add an output, add one entry to `ARTIFACTS`.
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { renderBlocksEmoji } from './generate/blocks-emoji.mjs';
import { renderBlocksSlash } from './generate/blocks-slash.mjs';
import { renderBlocksTypes } from './generate/blocks-types.mjs';
import { renderEguiTokens } from './generate/egui-tokens.mjs';
import { renderEguiPalette, renderTuiPalette } from './generate/rust-palette.mjs';
import { renderThemeTs } from './generate/theme-ts.mjs';
import { renderTokensCss } from './generate/tokens-css.mjs';

const REPO = dirname(dirname(fileURLToPath(import.meta.url)));

/** Every generated file: repo-relative path, and the function that emits its text. */
const ARTIFACTS = [
  { path: 'packages/tokens/css/tokens.css', render: renderTokensCss },
  { path: 'packages/tokens/src/theme.gen.ts', render: renderThemeTs },
  { path: 'crates/forge-tui/src/theme/palette.rs', render: renderTuiPalette },
  { path: 'crates/forge-egui/src/theme/palette.rs', render: renderEguiPalette },
  { path: 'crates/forge-egui/src/theme/tokens.rs', render: renderEguiTokens },
  { path: 'packages/blocks/src/types.gen.ts', render: renderBlocksTypes },
  { path: 'packages/blocks/src/slash.gen.ts', render: renderBlocksSlash },
  { path: 'packages/blocks/src/emoji.gen.ts', render: renderBlocksEmoji },
];

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
  console.error('These generated files no longer match their source:\n');
  for (const path of stale) console.error(`  ${path}`);
  console.error('\nRun `just generate` and commit the result.');
  return 1;
}

process.exitCode = await main(process.argv.includes('--check'));
