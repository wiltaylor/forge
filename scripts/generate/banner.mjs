/**
 * The header every generated artifact opens with.
 *
 * The lines are plain text; each generator wraps them in the comment syntax of
 * the file it emits. They say the file is generated and name the source, so a
 * reader who is about to hand-edit one is told where the edit belongs.
 */
import { SOURCE_PATH } from '../../packages/tokens/tokens.source.mjs';

/** @returns {string[]} the banner text, one entry per line. */
export function bannerLines() {
  return [
    'GENERATED FILE — do not edit by hand.',
    `Source:     ${SOURCE_PATH}`,
    'Regenerate: just generate   (`just check` fails while this file is stale)',
  ];
}
