/* Element → particle source pixels.

   Snapshot strategy: clone the element with computed styles inlined, wrap it
   in an SVG <foreignObject>, draw the serialized SVG onto an offscreen canvas
   and read the pixels back. SVG-as-image loads NO external subresources
   (cross-origin images render blank rather than tainting), so besides the
   try/catch around getImageData we also run a blankness check — a mostly
   transparent snapshot means the fallback path will look better than a
   handful of stray pixels. */

export interface PixelRaster {
  kind: 'pixels';
  /** Sampled particle positions in viewport CSS px + packed 0xAABBGGRR colors. */
  xs: Float32Array;
  ys: Float32Array;
  colors: Uint32Array;
  count: number;
  /** CSS px between samples — particles sized to this tile the element
      seamlessly instead of leaving gaps. */
  cell: number;
}

export interface FallbackRaster {
  kind: 'fallback';
}

export type Raster = PixelRaster | FallbackRaster;

const MAX_EDGE = 320;
const LOAD_TIMEOUT_MS = 300;
const MIN_ALPHA = 32;

function inlineStyles(source: Element, target: Element): void {
  if (source instanceof HTMLElement && target instanceof HTMLElement) {
    const computed = getComputedStyle(source);
    if (computed.cssText) {
      target.style.cssText = computed.cssText;
    } else {
      // Firefox: computed cssText is '' — copy property by property.
      for (let i = 0; i < computed.length; i++) {
        const prop = computed[i]!;
        target.style.setProperty(prop, computed.getPropertyValue(prop));
      }
    }
  }
  const s = source.children;
  const t = target.children;
  for (let i = 0; i < s.length; i++) inlineStyles(s[i]!, t[i]!);
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    const timer = setTimeout(() => reject(new Error('fx raster timeout')), LOAD_TIMEOUT_MS);
    img.onload = () => {
      clearTimeout(timer);
      resolve(img);
    };
    img.onerror = () => {
      clearTimeout(timer);
      reject(new Error('fx raster load error'));
    };
    img.src = src;
  });
}

/** Sample an element's rendered pixels into particle home positions.
    Never throws — any failure returns the fallback marker. */
export async function rasterizeElement(el: HTMLElement, budget: number): Promise<Raster> {
  try {
    const rect = el.getBoundingClientRect();
    const w = Math.round(rect.width);
    const h = Math.round(rect.height);
    if (w < 2 || h < 2 || budget < 1) return { kind: 'fallback' };

    const scale = Math.min(1, MAX_EDGE / Math.max(w, h));
    const clone = el.cloneNode(true) as HTMLElement;
    inlineStyles(el, clone);
    clone.style.margin = '0';

    const ns = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(ns, 'svg');
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    const fo = document.createElementNS(ns, 'foreignObject');
    fo.setAttribute('width', '100%');
    fo.setAttribute('height', '100%');
    const wrap = document.createElementNS('http://www.w3.org/1999/xhtml', 'div');
    wrap.appendChild(clone);
    fo.appendChild(wrap);
    svg.appendChild(fo);

    const uri = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`;
    const img = await loadImage(uri);

    const cw = Math.max(1, Math.round(w * scale));
    const ch = Math.max(1, Math.round(h * scale));
    const canvas = document.createElement('canvas');
    canvas.width = cw;
    canvas.height = ch;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return { kind: 'fallback' };
    ctx.drawImage(img, 0, 0, cw, ch);
    const data = ctx.getImageData(0, 0, cw, ch).data; // throws SecurityError if tainted

    // Sample on a stride grid sized to the particle budget.
    const stride = Math.max(1, Math.ceil(Math.sqrt((cw * ch) / budget)));
    const cap = Math.ceil(cw / stride) * Math.ceil(ch / stride);
    const xs = new Float32Array(cap);
    const ys = new Float32Array(cap);
    const colors = new Uint32Array(cap);
    let count = 0;
    let opaque = 0;
    const cell = stride / scale; // CSS px per sampled cell

    for (let py = 0; py < ch; py += stride) {
      for (let px = 0; px < cw; px += stride) {
        const o = (py * cw + px) * 4;
        const a = data[o + 3]!;
        if (a >= MIN_ALPHA) opaque++;
        if (a < MIN_ALPHA) continue;
        // px/py are scaled-canvas coords — un-scale back to CSS px. Adjacent
        // samples then sit exactly `cell` apart, so cell-sized particles tile
        // the element seamlessly.
        xs[count] = rect.left + (px + 0.5) / scale;
        ys[count] = rect.top + (py + 0.5) / scale;
        colors[count] = (a << 24) | (data[o + 2]! << 16) | (data[o + 1]! << 8) | data[o]!;
        count++;
      }
    }

    // Blankness check: SVG-as-image silently drops external subresources, so
    // an element dominated by a remote image comes back nearly empty.
    if (count < 4 || opaque / cap < 0.02) return { kind: 'fallback' };
    return { kind: 'pixels', xs, ys, colors, count, cell };
  } catch {
    return { kind: 'fallback' };
  }
}

function parseCssColor(value: string): number | undefined {
  const m = value.match(/rgba?\(\s*(\d+)[\s,]+(\d+)[\s,]+(\d+)(?:[\s,/]+([\d.]+))?\s*\)/);
  if (!m) return undefined;
  const a = m[4] === undefined ? 255 : Math.round(parseFloat(m[4]) * 255);
  if (a < MIN_ALPHA) return undefined;
  return (a << 24) | (Number(m[3]) << 16) | (Number(m[2]) << 8) | Number(m[1]);
}

/** Resolve any CSS color string (hex, oklch(), var-resolved…) to packed RGBA
    by letting the browser normalize it via a computed style. */
export function resolveColor(value: string): number | undefined {
  const probe = document.createElement('div');
  probe.style.color = value;
  probe.style.display = 'none';
  document.body.appendChild(probe);
  const packed = parseCssColor(getComputedStyle(probe).color);
  probe.remove();
  return packed;
}

/** Fallback particle grid over the element's rect, tinted from the given
    colors or the element's own computed colors + theme accent. */
export function fallbackRaster(el: HTMLElement, budget: number, colorInputs?: string[]): PixelRaster {
  const rect = el.getBoundingClientRect();
  const w = Math.max(2, rect.width);
  const h = Math.max(2, rect.height);

  const palette: number[] = [];
  const inputs = colorInputs?.length
    ? colorInputs
    : (() => {
        const cs = getComputedStyle(el);
        const root = getComputedStyle(document.documentElement);
        return [cs.backgroundColor, cs.color, root.getPropertyValue('--accent'), root.getPropertyValue('--accent-fg')];
      })();
  for (const input of inputs) {
    const packed = input ? resolveColor(input.trim()) : undefined;
    if (packed !== undefined && packed !== 0) palette.push(packed);
  }
  if (!palette.length) palette.push(0xffaaaaaa);

  const stride = Math.max(2, Math.ceil(Math.sqrt((w * h) / budget)));
  const cap = Math.ceil(w / stride) * Math.ceil(h / stride);
  const xs = new Float32Array(cap);
  const ys = new Float32Array(cap);
  const colors = new Uint32Array(cap);
  let count = 0;
  for (let py = 0; py < h; py += stride) {
    for (let px = 0; px < w; px += stride) {
      xs[count] = rect.left + px + stride / 2;
      ys[count] = rect.top + py + stride / 2;
      colors[count] = palette[count % palette.length]!;
      count++;
    }
  }
  return { kind: 'pixels', xs, ys, colors, count, cell: stride };
}
