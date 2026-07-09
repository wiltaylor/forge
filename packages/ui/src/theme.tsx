import { Show, createContext, createEffect, createSignal, useContext } from 'solid-js';
import type { Accessor, JSX } from 'solid-js';
import { applyTheme } from '@forge/tokens';
import type { Theme } from '@forge/tokens';

export type ThemeInput = Theme | 'dark' | 'light';

export interface ThemeContextValue {
  theme: Accessor<ThemeInput>;
  setTheme: (theme: ThemeInput) => void;
}

const ThemeContext = createContext<ThemeContextValue>();

export interface ThemeProviderProps {
  /** Initial theme, and — when changed reactively — the controlled theme. */
  theme?: ThemeInput;
  /**
   * When set, the theme applies only to this provider's subtree (renders a
   * wrapping div carrying data-theme + inline vars) instead of `<html>`.
   */
  scoped?: boolean;
  children?: JSX.Element;
}

/**
 * Applies the theme (globally on `<html>` by default; on a wrapper div when
 * `scoped`) and exposes it via `useTheme()`. Because everything routes
 * through CSS custom properties, a global apply also recolors remote
 * components mounted in shadow roots.
 */
export function ThemeProvider(props: ThemeProviderProps) {
  const [theme, setTheme] = createSignal<ThemeInput>(props.theme ?? 'dark');
  createEffect(() => { if (props.theme !== undefined) setTheme(() => props.theme!); });

  let scopeEl: HTMLDivElement | undefined;
  createEffect(() => {
    const t = theme();
    if (props.scoped) {
      if (scopeEl) applyTheme(t, scopeEl);
    } else {
      applyTheme(t);
    }
  });

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      <Show when={props.scoped} fallback={props.children}>
        <div ref={scopeEl}>{props.children}</div>
      </Show>
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used inside a <ThemeProvider>');
  return ctx;
}
