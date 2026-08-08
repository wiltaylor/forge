/**
 * The authored Forge token source — the one place a token value changes.
 *
 * A generator makes each palette from this file, and the tree holds the result.
 * Today that is the CSS custom properties in `css/tokens.css`, the typed theme
 * in `src/theme.gen.ts`, both Rust kit palettes, and forge-egui's geometry,
 * type and motion tokens. Run `just generate` after you edit this file.
 * `just check` fails while a generated file is stale.
 *
 * Shape
 * -----
 * `groups` is an ordered list of token groups. The order and the grouping are
 * the layout of the generated CSS: each group becomes a blank-line-separated
 * run of declarations, and a group's `comment` becomes the comment above it.
 * A group with no `tokens` is a comment-only note (the breakpoint block).
 *
 * A token holds either
 *   - `value` — one value for both schemes; declared in `:root` only, or
 *   - `dark` and `light` — a scheme token; declared in `:root` and repeated in
 *     each scheme-override block.
 *
 * A token belongs to every kit in `KITS` unless it names the ones it belongs to:
 *   { name: 'sidebar-rail-w', only: ['egui'], … }
 * `only` is for a dimension one kit has and the others have no equivalent of.
 * A token scoped away from the web kit gets no CSS custom property, and reading
 * a token from a kit it is not scoped to is an error rather than a silent value.
 *
 * Values
 * ------
 *   { hex: '#11141A' }                     an sRGB literal, as authored
 *   { oklch: [L, C, H] }                   an OKLCH colour
 *   { oklch: [L, C, H], alpha, over }      a translucent tint
 *   { raw: '4px' }                         verbatim CSS (sizes, motion, fonts)
 *
 * `alpha` and `over` are the derivation metadata for the tints. `alpha` is the
 * opacity the web paints the tint at. `over` names the token whose surface the
 * tint flattens over, for a target that has no alpha — the terminal kit, which
 * takes each tint already composited over the card. The desktop kit takes the
 * same tint with `alpha` quantised to a byte. Both rules are declared here and
 * run in `scripts/generate/oklch.mjs`, not in a comment in either kit.
 */

/** Repo-relative path of this file, for the "do not edit" header of every generated artifact. */
export const SOURCE_PATH = 'packages/tokens/tokens.source.mjs';

/** Title comment carried at the top of the generated stylesheet. */
export const CSS_TITLE = [
  'Forge Design System — Tokens',
  'Dark default, light via prefers-color-scheme or [data-theme="light"].',
];

/** The surface every translucent tint flattens over when the target has no alpha. */
const CARD = 'bg-1';

const hex = (value) => ({ hex: value });
const oklch = (l, c, h) => ({ oklch: [l, c, h] });
const tint = (l, c, h, alpha) => ({ oklch: [l, c, h], alpha, over: CARD });
const raw = (value) => ({ raw: value });

export const groups = [
  {
    comment: ['Neutrals — backgrounds rise from 0 (page) to 4 (popovers)'],
    tokens: [
      { name: 'bg-0', note: 'page', dark: hex('#0B0D10'), light: hex('#FAFAFA') },
      { name: 'bg-1', note: 'card', dark: hex('#11141A'), light: hex('#FFFFFF') },
      { name: 'bg-2', note: 'hover / nested card', dark: hex('#171B22'), light: hex('#F4F5F7') },
      { name: 'bg-3', note: 'pressed / active row', dark: hex('#1E232C'), light: hex('#EAECEF') },
      { name: 'bg-4', note: 'popover, dropdown', dark: hex('#252B36'), light: hex('#FFFFFF') },
    ],
  },
  {
    comment: ['Foregrounds — descending contrast'],
    tokens: [
      { name: 'fg-0', note: 'primary text', dark: hex('#ECEEF2'), light: hex('#0C0F14') },
      { name: 'fg-1', note: 'secondary text', dark: hex('#B7BDC8'), light: hex('#3D4654') },
      { name: 'fg-2', note: 'tertiary, captions', dark: hex('#7C8593'), light: hex('#6B7383') },
      { name: 'fg-3', note: 'disabled, placeholder', dark: hex('#4E5664'), light: hex('#A0A6B2') },
    ],
  },
  {
    comment: ['Borders'],
    tokens: [
      { name: 'border-subtle', dark: hex('#1A1F27'), light: hex('#EEF0F3') },
      { name: 'border', dark: hex('#262C36'), light: hex('#DCDFE4') },
      { name: 'border-strong', dark: hex('#3A4250'), light: hex('#B6BBC4') },
    ],
  },
  {
    comment: ['Accent — desaturated blue'],
    tokens: [
      { name: 'accent', dark: oklch(0.62, 0.16, 250), light: oklch(0.52, 0.18, 250) },
      { name: 'accent-hover', dark: oklch(0.66, 0.17, 250), light: oklch(0.46, 0.19, 250) },
      { name: 'accent-press', dark: oklch(0.56, 0.16, 250), light: oklch(0.4, 0.19, 250) },
      { name: 'accent-bg', dark: tint(0.62, 0.16, 250, 0.14), light: tint(0.55, 0.17, 250, 0.14) },
      { name: 'accent-fg', dark: oklch(0.82, 0.13, 250), light: oklch(0.38, 0.19, 250) },
      { name: 'accent-contrast', note: 'text on solid accent', dark: hex('#FFFFFF'), light: hex('#FFFFFF') },
    ],
  },
  {
    comment: ['Semantic — each has -bg (tint) and -fg (text on tint)'],
    tokens: [
      { name: 'success', dark: oklch(0.68, 0.14, 150), light: oklch(0.5, 0.15, 150) },
      { name: 'success-bg', dark: tint(0.68, 0.14, 150, 0.14), light: tint(0.55, 0.15, 150, 0.16) },
      { name: 'success-fg', dark: oklch(0.82, 0.16, 150), light: oklch(0.36, 0.14, 150) },
    ],
  },
  {
    tokens: [
      { name: 'warning', dark: oklch(0.78, 0.14, 75), light: oklch(0.62, 0.16, 70) },
      { name: 'warning-bg', dark: tint(0.78, 0.14, 75, 0.14), light: tint(0.65, 0.16, 70, 0.2) },
      { name: 'warning-fg', dark: oklch(0.86, 0.13, 80), light: oklch(0.4, 0.14, 60) },
    ],
  },
  {
    tokens: [
      { name: 'danger', dark: oklch(0.65, 0.2, 25), light: oklch(0.52, 0.22, 25) },
      { name: 'danger-bg', dark: tint(0.65, 0.2, 25, 0.14), light: tint(0.55, 0.21, 25, 0.14) },
      { name: 'danger-fg', dark: oklch(0.78, 0.16, 25), light: oklch(0.42, 0.2, 25) },
    ],
  },
  {
    tokens: [
      { name: 'info', dark: oklch(0.68, 0.13, 230), light: oklch(0.5, 0.14, 230) },
      { name: 'info-bg', dark: tint(0.68, 0.13, 230, 0.14), light: tint(0.55, 0.14, 230, 0.16) },
      { name: 'info-fg', dark: oklch(0.82, 0.12, 230), light: oklch(0.36, 0.13, 230) },
    ],
  },
  {
    comment: ['Typography'],
    tokens: [
      {
        name: 'font-sans',
        value: raw("'IBM Plex Sans', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif"),
      },
      {
        name: 'font-mono',
        value: raw("'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, Consolas, monospace"),
      },
    ],
  },
  {
    comment: ['Type scale — 1.2 ratio, anchored at 14px'],
    tokens: [
      { name: 'fs-xs', value: raw('11px') },
      { name: 'fs-sm', value: raw('12px') },
      { name: 'fs-base', value: raw('14px') },
      { name: 'fs-md', value: raw('16px') },
      { name: 'fs-lg', value: raw('18px') },
      { name: 'fs-xl', value: raw('22px') },
      { name: 'fs-2xl', value: raw('28px') },
      { name: 'fs-3xl', value: raw('34px') },
    ],
  },
  {
    tokens: [
      { name: 'lh-tight', value: raw('1.2') },
      { name: 'lh-snug', value: raw('1.4') },
      { name: 'lh-normal', value: raw('1.5') },
      { name: 'lh-relaxed', value: raw('1.65') },
    ],
  },
  {
    tokens: [
      { name: 'fw-regular', value: raw('400') },
      { name: 'fw-medium', value: raw('500') },
      { name: 'fw-semibold', value: raw('600') },
      { name: 'fw-bold', value: raw('700') },
    ],
  },
  {
    tokens: [
      { name: 'tracking-tight', value: raw('-0.01em') },
      { name: 'tracking-normal', value: raw('0') },
      { name: 'tracking-wide', value: raw('0.04em') },
      { name: 'tracking-eyebrow', value: raw('0.08em') },
    ],
  },
  {
    comment: ['Spacing — 4px base'],
    tokens: [
      { name: 'sp-1', value: raw('4px') },
      { name: 'sp-2', value: raw('8px') },
      { name: 'sp-3', value: raw('12px') },
      { name: 'sp-4', value: raw('16px') },
      { name: 'sp-5', value: raw('20px') },
      { name: 'sp-6', value: raw('24px') },
      { name: 'sp-8', value: raw('32px') },
      { name: 'sp-10', value: raw('40px') },
      { name: 'sp-12', value: raw('48px') },
      { name: 'sp-16', value: raw('64px') },
    ],
  },
  {
    comment: ['Radii'],
    tokens: [
      { name: 'r-sm', value: raw('4px') },
      { name: 'r-md', value: raw('6px') },
      { name: 'r-lg', value: raw('8px') },
      { name: 'r-pill', value: raw('999px') },
    ],
  },
  {
    comment: ['Shadows — sparingly'],
    tokens: [
      { name: 'shadow-sm', dark: raw('none'), light: raw('none') },
      { name: 'shadow-md', dark: raw('none'), light: raw('none') },
    ],
  },
  {
    comment: ['Motion'],
    tokens: [
      { name: 'ease-out', value: raw('cubic-bezier(0.2, 0, 0, 1)') },
      { name: 'dur-1', value: raw('80ms') },
      { name: 'dur-2', value: raw('160ms') },
      { name: 'dur-3', value: raw('240ms') },
    ],
  },
  {
    comment: [
      'Responsive breakpoints — CSS variables cannot be used inside @media',
      'conditions, so these are documented constants. Use the literal values:',
      '  compact: @media (max-width: 1024px)  — sidebar becomes a drawer',
      '  mobile:  @media (max-width: 768px)   — single-column stacking',
      'Touch: @media (pointer: coarse) bumps the --h-* control heights below.',
    ],
  },
  {
    comment: ['Component sizes (touch-friendly minimums)'],
    tokens: [
      { name: 'h-sm', value: raw('28px') },
      { name: 'h-md', value: raw('32px') },
      { name: 'h-lg', value: raw('36px') },
      { name: 'h-xl', value: raw('40px') },
    ],
  },
  {
    comment: ['Shell dimensions — the app-shell grid and the mobile drawer share these'],
    tokens: [
      { name: 'sidebar-w', value: raw('240px') },
      { name: 'topbar-h', value: raw('48px') },
      // The desktop shell collapses its sidebar to an icon rail and carries a
      // status bar along the bottom. The web shell does neither, so these two
      // are scoped to that kit rather than declared as custom properties.
      { name: 'sidebar-rail-w', note: 'collapsed sidebar', only: ['egui'], value: raw('56px') },
      { name: 'statusbar-h', only: ['egui'], value: raw('28px') },
    ],
  },
  {
    comment: [
      'Layering — the z-index scale of the shell and its overlays, bottom to',
      'top. Every surface on this ladder takes its layer from here rather than',
      'declaring its own number; stacking local to one component (grid cells,',
      'drag ghosts) stays in that component\'s stylesheet. The modal and the',
      'command palette share the modal layer: both are modal surfaces, so only',
      'one of them is open at a time.',
    ],
    tokens: [
      { name: 'layer-topbar', note: 'sticky shell chrome', only: ['web'], value: raw('10') },
      { name: 'layer-scrim', note: 'backdrop behind the drawer', only: ['web'], value: raw('20') },
      { name: 'layer-drawer', note: 'off-canvas sidebar', only: ['web'], value: raw('30') },
      { name: 'layer-sheet', note: 'above the drawer, below the modal', only: ['web'], value: raw('40') },
      { name: 'layer-modal', note: 'modal and command palette', only: ['web'], value: raw('50') },
      { name: 'layer-pop', note: 'anchored popovers; above the modal so they work inside one', only: ['web'], value: raw('60') },
      { name: 'layer-toast', note: 'above the modal', only: ['web'], value: raw('70') },
      { name: 'layer-tip', note: 'tooltips top every interactive surface', only: ['web'], value: raw('80') },
      { name: 'layer-fx', note: 'non-interactive particle canvas', only: ['web'], value: raw('90') },
    ],
  },
];

/** Every token, in declaration order, flattened out of the groups. */
export const tokens = groups.flatMap((group) => group.tokens ?? []);

/** True when the token is declared per scheme rather than once for both. */
export const isSchemeToken = (token) => token.dark !== undefined;

/** Every kit a token can be scoped to. The stylesheet and the typed theme are `web`. */
export const KITS = ['web', 'tui', 'egui'];

/** True when the kit declares this token — every kit, unless the token says otherwise. */
export const inKit = (token, kit) => token.only === undefined || token.only.includes(kit);

/**
 * A misspelt kit would scope a token to nothing at all: it would vanish from
 * the stylesheet, and no generator would claim it. Refuse at load instead.
 */
for (const token of tokens) {
  for (const kit of token.only ?? []) {
    if (!KITS.includes(kit)) {
      throw new Error(`"${token.name}" is scoped to "${kit}", which is not one of ${KITS.join(', ')}`);
    }
  }
}

const byName = new Map(tokens.map((token) => [token.name, token]));

/**
 * The token of that name. Throws rather than emitting a palette with a hole in
 * it. Naming a kit also refuses a token that kit does not declare, so a layout
 * cannot quietly read another kit's dimension.
 */
export function tokenNamed(name, kit = undefined) {
  const token = byName.get(name);
  if (!token) throw new Error(`no token named "${name}" in ${SOURCE_PATH}`);
  if (kit !== undefined && !inKit(token, kit)) {
    throw new Error(`"${name}" is scoped to ${token.only.join(', ')}, so the ${kit} kit cannot read it`);
  }
  return token;
}

/** The value a token takes in a scheme. */
export const valueFor = (token, scheme) => (isSchemeToken(token) ? token[scheme] : token.value);
