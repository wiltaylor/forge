/**
 * Emit the CSS custom properties from the token source.
 *
 * The stylesheet states each scheme in more than one block — a `:root` default,
 * the `prefers-color-scheme` block, and a `[data-theme]` block per scheme, so
 * that preview panes can show both schemes at once. Every one of those blocks
 * is emitted from the same source entry, which is what stops the repeated
 * ramps drifting against each other.
 */
import { CSS_TITLE, groups, isSchemeToken, valueFor } from '../../packages/tokens/tokens.source.mjs';
import { bannerLines } from './banner.mjs';

const RULE = '='.repeat(73);

/** Decimals the author wrote, so `0.625` survives and `0.4` still prints as `0.40`. */
const authored = (n) => (String(n).split('.')[1] ?? '').length;
const fixed = (n, min) => n.toFixed(Math.max(min, authored(n)));

/** Render one authored value as the CSS text that declares it. */
export function formatValue(value) {
  if (value.hex !== undefined) return value.hex;
  if (value.raw !== undefined) return value.raw;
  const [l, c, h] = value.oklch;
  const alpha = value.alpha === undefined ? '' : ` / ${fixed(value.alpha, 2)}`;
  return `oklch(${fixed(l, 2)} ${fixed(c, 2)} ${fixed(h, 0)}${alpha})`;
}

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
 * `full` renders every token with its comments — the `:root` default. Otherwise
 * only the per-scheme tokens are rendered, which is what an override block
 * restates.
 */
function declarations(scheme, { indent, full }) {
  const blocks = [];
  for (const group of groups) {
    const tokens = (group.tokens ?? []).filter((t) => full || isSchemeToken(t));
    if (!tokens.length && !(full && group.comment)) continue;

    const lines = full && group.comment ? comment(group.comment, indent) : [];
    const labels = tokens.map((t) => `--${t.name}:`);
    const labelWidth = Math.max(0, ...labels.map((l) => l.length));
    const decls = tokens.map(
      (t, i) => `${indent}${labels[i].padEnd(labelWidth)} ${formatValue(valueFor(t, scheme))};`,
    );
    const declWidth = Math.max(0, ...decls.map((d) => d.length));
    tokens.forEach((t, i) => {
      const note = full && t.note ? `${' '.repeat(declWidth - decls[i].length + 2)}/* ${t.note} */` : '';
      lines.push(decls[i] + note);
    });
    blocks.push(lines.join('\n'));
  }
  return blocks.join('\n\n');
}

function block(selector, scheme, { indent = '  ', full = false } = {}) {
  const outer = indent.slice(2);
  return [
    `${outer}${selector} {`,
    `${indent}color-scheme: ${scheme};`,
    '',
    declarations(scheme, { indent, full }),
    `${outer}}`,
  ].join('\n');
}

/** @returns {string} the full text of `packages/tokens/css/tokens.css`. */
export function renderTokensCss() {
  const banner = [`/* ${RULE}`, ...bannerLines().map((l) => `   ${l}`), `   ${RULE} */`];
  const title = [`/* ${RULE}`, ...CSS_TITLE.map((l) => `   ${l}`), `   ${RULE} */`];

  return [
    ...banner,
    ...title,
    '/* -------- DARK (default) -------------------------------------------------- */',
    block(':root', 'dark', { full: true }),
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
