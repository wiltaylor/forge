/**
 * The header every generated artifact opens with.
 *
 * The lines are plain text; each generator wraps them in the comment syntax of
 * the file it emits. They say the file is generated and name the source, so a
 * reader who is about to hand-edit one is told where the edit belongs.
 */
import { SOURCE_PATH } from '../../packages/tokens/tokens.source.mjs';

/**
 * @param {string} [source] the authored file the artifact derives from.
 * @param {string} [via] the file the generator actually reads, when the source
 *   is not one Node can read — the block registry is Rust, dumped to JSON.
 * @returns {string[]} the banner text, one entry per line.
 */
export function bannerLines(source = SOURCE_PATH, via = undefined) {
  return [
    'GENERATED FILE — do not edit by hand.',
    `Source:     ${source}`,
    ...(via ? [`Read from:  ${via}   (\`just generate-blocks\` rewrites it)`] : []),
    'Regenerate: just generate   (`just check` fails while this file is stale)',
  ];
}
