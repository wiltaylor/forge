/**
 * Forge theme engine.
 *
 * The token values themselves are generated: `./theme.gen.ts` holds the `Theme`
 * type, the two built-in ramps and `themeToVars`, all from the token source.
 * This module is the behaviour around them, and re-exports the whole of it, so
 * `@forge/tokens` stays one import.
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
import { themeToVars } from './theme.gen';
import type { Theme } from './theme.gen';

export * from './theme.gen';

export type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends readonly unknown[]
    ? T[K]
    : T[K] extends object
      ? DeepPartial<T[K]>
      : T[K];
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
