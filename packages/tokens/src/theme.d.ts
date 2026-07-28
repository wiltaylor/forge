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
    border: {
        subtle: string;
        default: string;
        strong: string;
    };
    accent: ThemeAccent;
    success: SemanticTriple;
    warning: SemanticTriple;
    danger: SemanticTriple;
    info: SemanticTriple;
    fonts?: {
        sans?: string;
        mono?: string;
    };
    /** Escape hatch: any additional custom properties to set verbatim. */
    vars?: Record<`--${string}`, string>;
}
export type DeepPartial<T> = {
    [K in keyof T]?: T[K] extends readonly unknown[] ? T[K] : T[K] extends object ? DeepPartial<T[K]> : T[K];
};
/** The built-in dark ramp — mirrors the `:root` block in tokens.css. */
export declare const darkTheme: Theme;
/** The built-in light ramp — mirrors the `[data-theme="light"]` block in tokens.css. */
export declare const lightTheme: Theme;
/** Derive a new theme from a base, deep-merging overrides. */
export declare function defineTheme(base: Theme, overrides: DeepPartial<Theme>): Theme;
/** Flatten a Theme into the CSS custom-property map the stylesheets consume. */
export declare function themeToVars(t: Theme): Record<string, string>;
/**
 * Apply a theme globally (default: `<html>`) or to a subtree root.
 *
 * Strings apply the built-in ramps by setting `data-theme`; Theme objects
 * additionally write inline custom properties (clearing any from a previous
 * apply on the same element).
 */
export declare function applyTheme(theme: Theme | 'dark' | 'light', el?: HTMLElement): void;
