/**
 * Forge theme engine.
 *
 * The CSS files (tokens.css) define the dark and light ramps via `:root` and
 * `[data-theme]` blocks. This module layers a typed API on top:
 *
 * - `applyTheme('dark' | 'light', el?)` just sets `data-theme` and lets the
 *   stylesheet blocks do the work.
 * - `applyTheme(customTheme, el?)` sets `data-theme` to the theme's base
 *   scheme AND writes inline CSS custom properties, which outrank the
 *   `[data-theme]` selector blocks. Previously written inline vars are
 *   tracked per-element and cleared on the next apply.
 *
 * Custom properties inherit through shadow DOM boundaries, so applying a
 * theme on `document.documentElement` also restyles remote components
 * mounted in shadow roots.
 */

export interface SemanticTriple {
  /** Solid tone colour (borders, icons, strokes). */
  base: string;
  /** Translucent tint used as a background. */
  bg: string;
  /** Text colour readable on the tint. */
  fg: string;
}

export interface ThemeAccent {
  base: string;
  hover: string;
  press: string;
  bg: string;
  fg: string;
  /** Text on solid accent surfaces. */
  contrast: string;
}

export interface Theme {
  name: string;
  /** Base scheme the theme derives from — controls `data-theme` + `color-scheme`. */
  scheme: 'dark' | 'light';
  /** Backgrounds, page (0) → popover (4). Maps to --bg-0..--bg-4. */
  bg: [string, string, string, string, string];
  /** Foregrounds, primary (0) → disabled (3). Maps to --fg-0..--fg-3. */
  fg: [string, string, string, string];
  border: { subtle: string; default: string; strong: string };
  accent: ThemeAccent;
  success: SemanticTriple;
  warning: SemanticTriple;
  danger: SemanticTriple;
  info: SemanticTriple;
  fonts?: { sans?: string; mono?: string };
  /** Escape hatch: any additional custom properties to set verbatim. */
  vars?: Record<`--${string}`, string>;
}

export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends readonly unknown[]
    ? T[K]
    : T[K] extends object
      ? DeepPartial<T[K]>
      : T[K];
};

/** The built-in dark ramp — mirrors the `:root` block in tokens.css. */
export const darkTheme: Theme = {
  name: 'forge-dark',
  scheme: 'dark',
  bg: ['#0B0D10', '#11141A', '#171B22', '#1E232C', '#252B36'],
  fg: ['#ECEEF2', '#B7BDC8', '#7C8593', '#4E5664'],
  border: { subtle: '#1A1F27', default: '#262C36', strong: '#3A4250' },
  accent: {
    base: 'oklch(0.62 0.16 250)',
    hover: 'oklch(0.66 0.17 250)',
    press: 'oklch(0.56 0.16 250)',
    bg: 'oklch(0.62 0.16 250 / 0.14)',
    fg: 'oklch(0.82 0.13 250)',
    contrast: '#FFFFFF',
  },
  success: { base: 'oklch(0.68 0.14 150)', bg: 'oklch(0.68 0.14 150 / 0.14)', fg: 'oklch(0.82 0.16 150)' },
  warning: { base: 'oklch(0.78 0.14 75)', bg: 'oklch(0.78 0.14 75 / 0.14)', fg: 'oklch(0.86 0.13 80)' },
  danger: { base: 'oklch(0.65 0.20 25)', bg: 'oklch(0.65 0.20 25 / 0.14)', fg: 'oklch(0.78 0.16 25)' },
  info: { base: 'oklch(0.68 0.13 230)', bg: 'oklch(0.68 0.13 230 / 0.14)', fg: 'oklch(0.82 0.12 230)' },
};

/** The built-in light ramp — mirrors the `[data-theme="light"]` block in tokens.css. */
export const lightTheme: Theme = {
  name: 'forge-light',
  scheme: 'light',
  bg: ['#FAFAFA', '#FFFFFF', '#F4F5F7', '#EAECEF', '#FFFFFF'],
  fg: ['#0C0F14', '#3D4654', '#6B7383', '#A0A6B2'],
  border: { subtle: '#EEF0F3', default: '#DCDFE4', strong: '#B6BBC4' },
  accent: {
    base: 'oklch(0.52 0.18 250)',
    hover: 'oklch(0.46 0.19 250)',
    press: 'oklch(0.40 0.19 250)',
    bg: 'oklch(0.55 0.17 250 / 0.14)',
    fg: 'oklch(0.38 0.19 250)',
    contrast: '#FFFFFF',
  },
  success: { base: 'oklch(0.50 0.15 150)', bg: 'oklch(0.55 0.15 150 / 0.16)', fg: 'oklch(0.36 0.14 150)' },
  warning: { base: 'oklch(0.62 0.16 70)', bg: 'oklch(0.65 0.16 70 / 0.20)', fg: 'oklch(0.40 0.14 60)' },
  danger: { base: 'oklch(0.52 0.22 25)', bg: 'oklch(0.55 0.21 25 / 0.14)', fg: 'oklch(0.42 0.20 25)' },
  info: { base: 'oklch(0.50 0.14 230)', bg: 'oklch(0.55 0.14 230 / 0.16)', fg: 'oklch(0.36 0.13 230)' },
};

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function merge<T>(base: T, overrides: DeepPartial<T>): T {
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(overrides as Record<string, unknown>)) {
    if (v === undefined) continue;
    const cur = out[k];
    out[k] = isObject(cur) && isObject(v) ? merge(cur, v) : v;
  }
  return out as T;
}

/** Derive a new theme from a base, deep-merging overrides. */
export function defineTheme(base: Theme, overrides: DeepPartial<Theme>): Theme {
  return merge(base, overrides);
}

/** Flatten a Theme into the CSS custom-property map the stylesheets consume. */
export function themeToVars(t: Theme): Record<string, string> {
  const vars: Record<string, string> = {};
  t.bg.forEach((v, i) => (vars[`--bg-${i}`] = v));
  t.fg.forEach((v, i) => (vars[`--fg-${i}`] = v));
  vars['--border-subtle'] = t.border.subtle;
  vars['--border'] = t.border.default;
  vars['--border-strong'] = t.border.strong;
  vars['--accent'] = t.accent.base;
  vars['--accent-hover'] = t.accent.hover;
  vars['--accent-press'] = t.accent.press;
  vars['--accent-bg'] = t.accent.bg;
  vars['--accent-fg'] = t.accent.fg;
  vars['--accent-contrast'] = t.accent.contrast;
  for (const tone of ['success', 'warning', 'danger', 'info'] as const) {
    const triple = t[tone];
    vars[`--${tone}`] = triple.base;
    vars[`--${tone}-bg`] = triple.bg;
    vars[`--${tone}-fg`] = triple.fg;
  }
  if (t.fonts?.sans) vars['--font-sans'] = t.fonts.sans;
  if (t.fonts?.mono) vars['--font-mono'] = t.fonts.mono;
  if (t.vars) Object.assign(vars, t.vars);
  return vars;
}

/** Inline vars written by applyTheme, tracked per element so the next apply clears them. */
const appliedVars = new WeakMap<HTMLElement, string[]>();

/**
 * Apply a theme globally (default: `<html>`) or to a subtree root.
 *
 * Strings apply the built-in ramps by setting `data-theme`; Theme objects
 * additionally write inline custom properties (clearing any from a previous
 * apply on the same element).
 */
export function applyTheme(theme: Theme | 'dark' | 'light', el?: HTMLElement): void {
  const target = el ?? document.documentElement;
  for (const name of appliedVars.get(target) ?? []) target.style.removeProperty(name);
  appliedVars.delete(target);

  if (typeof theme === 'string') {
    target.setAttribute('data-theme', theme);
    return;
  }

  target.setAttribute('data-theme', theme.scheme);
  const vars = themeToVars(theme);
  for (const [name, value] of Object.entries(vars)) target.style.setProperty(name, value);
  appliedVars.set(target, Object.keys(vars));
}
