/**
 * Emit the emoji shortcode table.
 *
 * The table used to live twice — once in Rust, once in TypeScript — under a
 * comment asking whoever edited one to remember the other. The Rust table is
 * the author now; this writes its TypeScript form.
 *
 * The order is the table's own, sorted by shortcode. `searchEmoji` walks the
 * object in key order, so it must stay sorted here too.
 */
import { bannerLines } from './banner.mjs';
import { EMOJI_PATH, EMOJI_SOURCE_PATH, emoji, via } from './blocks-source.mjs';
import { quote } from './ts.mjs';

/** The whole file. */
export function renderBlocksEmoji() {
  const lines = [
    `/* ${bannerLines(EMOJI_SOURCE_PATH, via(EMOJI_PATH)).join('\n   ')} */`,
    '',
    '/** Curated emoji shortcode table — gemoji-compatible names, sorted by',
    '    shortcode. */',
    'export const EMOJI: Record<string, string> = {',
    ...emoji.map(([code, glyph]) => `  ${quote(code)}: ${quote(glyph)},`),
    '};',
  ];
  return `${lines.join('\n')}\n`;
}
