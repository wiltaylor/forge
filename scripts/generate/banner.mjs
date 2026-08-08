/**
 * The header every generated artifact opens with.
 *
 * The lines are plain text; each generator wraps them in the comment syntax of
 * the file it emits. They say the file is generated and name the source, so a
 * reader who is about to hand-edit one is told where the edit belongs.
 */
import { SOURCE_PATH } from '../../packages/tokens/tokens.source.mjs';

/**
 * @param {string | string[]} [source] the authored file(s) the artifact
 *   derives from — a list renders one per line under the `Source:` label.
 * @param {string} [via] what the generator actually reads, when the source is
 *   not a file Node can read — the block registry is Rust, dumped to JSON.
 * @returns {string[]} the banner text, one entry per line.
 */
export function bannerLines(source = SOURCE_PATH, via = undefined) {
  const label = 'Source:     ';
  const [first, ...rest] = Array.isArray(source) ? source : [source];
  return [
    'GENERATED FILE — do not edit by hand.',
    `${label}${first}`,
    ...rest.map((s) => `${' '.repeat(label.length)}${s}`),
    ...(via ? [`Read from:  ${via}`] : []),
    'Regenerate: just generate   (`just check` fails while this file is stale)',
  ];
}

/** Wrap banner lines in the ruled box comment a generated stylesheet opens with. */
export const boxedCssComment = (lines) => {
  const rule = '='.repeat(73);
  return [`/* ${rule}`, ...lines.map((l) => `   ${l}`), `   ${rule} */`];
};
