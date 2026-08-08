/* Emoji shortcodes. The table is generated from crates/forge-blocks/src/emoji.rs
   (`emoji.gen.ts`); resolving and searching it is here. */
import { EMOJI } from './emoji.gen';

export { EMOJI } from './emoji.gen';

const SHORTCODE = /:([a-z0-9_+-]+):/g;

/** Replace every known `:shortcode:` with its emoji; unknown codes stay
    literal. Apply to plain-text runs only (never code spans/blocks). */
export function resolveEmoji(text: string, extra?: Record<string, string>): string {
  if (!text.includes(':')) return text;
  return text.replace(SHORTCODE, (m, code: string) => extra?.[code] ?? EMOJI[code] ?? m);
}

/** Shortcodes starting with `prefix`, for the `:xx` autocomplete popup. */
export function searchEmoji(
  prefix: string,
  extra?: Record<string, string>,
  limit = 8,
): { code: string; char: string }[] {
  const out: { code: string; char: string }[] = [];
  if (extra)
    for (const code of Object.keys(extra).sort()) {
      if (out.length >= limit) return out;
      if (code.startsWith(prefix)) out.push({ code, char: extra[code]! });
    }
  for (const code of Object.keys(EMOJI)) {
    if (out.length >= limit) break;
    if (code.startsWith(prefix) && !extra?.[code]) out.push({ code, char: EMOJI[code]! });
  }
  return out;
}
