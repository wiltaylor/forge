/* ---------------- Terminal theme from Forge tokens --------------------------
   xterm paints to canvas, so `var(--token)` strings can't be used directly:
   token values are resolved to concrete colors at read time and re-read on
   theme change (watchTheme).

   The tokens ship no ANSI palette, so the 16 ANSI colors are a documented
   approximation: red/green/yellow/blue map to danger/success/warning/accent
   (bright = the matching -fg variants), magenta/cyan are oklch midpoints of
   danger+accent / info+success, and the gray slots reuse bg/fg shades. */
import type { ITheme } from '@xterm/xterm';

let probe: HTMLSpanElement | undefined;
let ctx: CanvasRenderingContext2D | null | undefined;

/** Resolve any CSS color expression (var(), color-mix(), oklch()) in the
    context of `el` to a color string xterm's parser accepts (hex/rgba). */
function resolve(el: Element, expr: string, fallback: string): string {
  if (!probe) {
    probe = document.createElement('span');
    probe.style.display = 'none';
  }
  el.appendChild(probe);
  probe.style.color = expr;
  const computed = getComputedStyle(probe).color;
  probe.remove();
  /* Round-trip through canvas fillStyle: browsers may serialize computed
     colors as oklch()/color(), which xterm can't parse; fillStyle always
     reads back as #rrggbb or rgba(). */
  if (ctx === undefined) ctx = document.createElement('canvas').getContext('2d');
  if (!ctx) return fallback;
  ctx.fillStyle = fallback;
  ctx.fillStyle = computed;
  return ctx.fillStyle;
}

const mix = (a: string, b: string) => `color-mix(in oklch, var(${a}) 50%, var(${b}))`;

/** Read the current Forge tokens into an xterm ITheme (concrete colors). */
export function readTermTheme(el: Element): ITheme {
  const c = (expr: string, fallback = '#000000') => resolve(el, expr, fallback);
  const v = (name: string, fallback?: string) => c(`var(${name})`, fallback);
  return {
    background: v('--bg-0', '#0B0D10'),
    foreground: v('--fg-0', '#ECEEF2'),
    cursor: v('--accent', '#5A8FDB'),
    cursorAccent: v('--bg-0', '#0B0D10'),
    selectionBackground: v('--accent-bg', 'rgba(90,143,219,0.14)'),
    black: v('--bg-3'),
    red: v('--danger'),
    green: v('--success'),
    yellow: v('--warning'),
    blue: v('--accent'),
    magenta: c(mix('--danger', '--accent')),
    cyan: c(mix('--info', '--success')),
    white: v('--fg-1'),
    brightBlack: v('--fg-3'),
    brightRed: v('--danger-fg'),
    brightGreen: v('--success-fg'),
    brightYellow: v('--warning-fg'),
    brightBlue: v('--accent-fg'),
    brightMagenta: c(mix('--danger-fg', '--accent-fg')),
    brightCyan: c(mix('--info-fg', '--success-fg')),
    brightWhite: v('--fg-0'),
  };
}

/** Re-run `cb` when the gallery/app toggles theme (writes `data-theme` or an
    inline style on documentElement). Returns a dispose function. */
export function watchTheme(cb: () => void): () => void {
  const mo = new MutationObserver(cb);
  mo.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme', 'style'],
  });
  return () => mo.disconnect();
}
