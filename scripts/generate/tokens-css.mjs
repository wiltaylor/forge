/**
 * Emit the CSS custom properties from the token source.
 *
 * The stylesheet states each scheme in more than one block. A `:root` default,
 * the `prefers-color-scheme` block, and one `[data-theme]` block per scheme let
 * a preview pane show both schemes at once. Each of those blocks comes from the
 * same source entry, so the repeated ramps cannot drift apart.
 */
import { CSS_TITLE, groups, inKit, isSchemeToken, valueFor } from '../../packages/tokens/tokens.source.mjs';
import { bannerLines } from './banner.mjs';

const RULE = '='.repeat(73);

/** Count the decimals the author wrote. */
const authoredDecimals = (n) => (String(n).split('.')[1] ?? '').length;

/** Print a number with at least `min` decimals, and with every decimal the author wrote. */
const number = (n, min) => n.toFixed(Math.max(min, authoredDecimals(n)));

/** Render one authored value as the CSS text that declares it. */
export function formatValue(value) {
  if (value.hex !== undefined) return value.hex;
  if (value.raw !== undefined) return value.raw;
  const [l, c, h] = value.oklch;
  const alpha = value.alpha === undefined ? '' : ` / ${number(value.alpha, 2)}`;
  return `oklch(${number(l, 2)} ${number(c, 2)} ${number(h, 0)}${alpha})`;
}

/** Wrap lines in the ruled box comment the stylesheet opens its sections with. */
const boxed = (lines) => [`/* ${RULE}`, ...lines.map((l) => `   ${l}`), `   ${RULE} */`];

function comment(lines, indent) {
  if (lines.length === 1) return [`${indent}/* ${lines[0]} */`];
  const continuation = `${indent}   `;
  return lines.map((line, i) => {
    const open = i === 0 ? `${indent}/* ` : continuation;
    const close = i === lines.length - 1 ? ' */' : '';
    return `${open}${line}${close}`;
  });
}

/**
 * Render the declarations of a scheme.
 *
 * `wholeSet` renders every token with its comments — the `:root` default. The
 * override blocks restate the per-scheme tokens only, and without the comments.
 * A token scoped to another kit is declared in neither: the stylesheet carries
 * the web kit's tokens.
 */
function declarations(scheme, { indent, wholeSet }) {
  const blocks = [];
  for (const group of groups) {
    const tokens = (group.tokens ?? [])
      .filter((t) => inKit(t, 'web'))
      .filter((t) => wholeSet || isSchemeToken(t));
    if (!tokens.length && !(wholeSet && group.comment)) continue;

    const lines = wholeSet && group.comment ? comment(group.comment, indent) : [];
    const labels = tokens.map((t) => `--${t.name}:`);
    const labelWidth = Math.max(0, ...labels.map((l) => l.length));
    const decls = tokens.map(
      (t, i) => `${indent}${labels[i].padEnd(labelWidth)} ${formatValue(valueFor(t, scheme))};`,
    );
    const declWidth = Math.max(0, ...decls.map((d) => d.length));
    tokens.forEach((t, i) => {
      const pad = ' '.repeat(declWidth - decls[i].length + 2);
      lines.push(decls[i] + (wholeSet && t.note ? `${pad}/* ${t.note} */` : ''));
    });
    blocks.push(lines.join('\n'));
  }
  return blocks.join('\n\n');
}

function block(selector, scheme, { indent = '  ', wholeSet = false } = {}) {
  // The selector sits one level out from the declarations it opens.
  const selectorIndent = indent.slice(2);
  return [
    `${selectorIndent}${selector} {`,
    `${indent}color-scheme: ${scheme};`,
    '',
    declarations(scheme, { indent, wholeSet }),
    `${selectorIndent}}`,
  ].join('\n');
}

/** @returns {string} the full text of `packages/tokens/css/tokens.css`. */
export function renderTokensCss() {
  return [
    ...boxed(bannerLines()),
    ...boxed(CSS_TITLE),
    '/* -------- DARK (default) -------------------------------------------------- */',
    block(':root', 'dark', { wholeSet: true }),
    '',
    '/* -------- LIGHT ----------------------------------------------------------- */',
    '@media (prefers-color-scheme: light) {',
    block(':root:not([data-theme="dark"])', 'light', { indent: '    ' }),
    '}',
    '',
    '/* Manual theme overrides — applies to ANY element with data-theme,',
    '   so multi-theme preview panes can show both at once. */',
    block('[data-theme="light"]', 'light'),
    '',
    '/* Explicit dark override — re-asserts the dark ramp when the parent is light',
    '   (e.g. side-by-side preview panes, or when the OS prefers light but a',
    '   surface needs to stay dark). */',
    block('[data-theme="dark"]', 'dark'),
    '',
  ].join('\n');
}
