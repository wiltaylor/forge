/* Particle FX — imperative effects for item creation / destruction feedback.

   Mount <FxLayer /> once at the app root (optional — the canvas is appended
   to document.body on first use otherwise), then call fx.explode(el) /
   fx.recreate(el) / fx.materialize(el) / fx.burst(el) from anywhere. The
   element is never removed from the DOM: hiding is done via inline
   `visibility` and always restored (except explode, whose end state is
   "destroyed" — hidden until the caller decides otherwise).

   Like the toaster, the store lives on `globalThis` keyed by a versioned
   symbol so host app + remote web-component bundles share ONE canvas and one
   degradation state. Keep the store shape stable within the `v1` key.

   Every trigger re-checks capability/accessibility (prefers-reduced-motion,
   low-core/low-memory devices, session slow-frame downgrade) and degrades to
   a short opacity fade — callers never need to branch. */

import { onMount } from 'solid-js';
import type { JSX } from 'solid-js';
import { FxEngine } from './engine';
import type { FxPhase } from './engine';
import { fallbackRaster, rasterizeElement, resolveColor } from './rasterize';
import type { PixelRaster } from './rasterize';
import { detectFxTier as detectTier, GLOBAL_BUDGET, tierBudget, tierDurationScale } from './tier';
import type { FxGlobalConfig, FxMode, FxTier } from './tier';
import { useOverlayMount } from '../overlay-mount';

export type { FxMode, FxTier };

export interface FxOptions {
  /** Fallback + burst palette; defaults to element colors / theme accent. */
  colors?: string[];
  /** ms for the main phase (burst or converge); defaults per effect. */
  duration?: number;
  /** px/s². Default 900 (0 for materialize). */
  gravity?: number;
  /** Initial velocity scale. Default 1. */
  spread?: number;
  /** Per-effect particle cap on top of the tier budget. */
  maxParticles?: number;
  /** Particle square size in CSS px. Default 3. */
  particleSize?: number;
}

interface FxStore extends FxGlobalConfig {
  engine?: FxEngine;
}

const KEY = Symbol.for('forge.fx.v1');

function store(): FxStore {
  const g = globalThis as Record<symbol, unknown>;
  if (!g[KEY]) g[KEY] = { mode: 'auto', degraded: false, maxParticles: Infinity } satisfies FxStore;
  return g[KEY] as FxStore;
}

function engine(): FxEngine {
  const s = store();
  s.engine ??= new FxEngine(() => {
    s.degraded = true;
  });
  return s.engine;
}

/** Configure the shared FX system ('auto' applies live detection). */
export function fxConfig(cfg: { mode?: FxMode; maxParticles?: number }): void {
  const s = store();
  if (cfg.mode) s.mode = cfg.mode;
  if (cfg.maxParticles !== undefined) s.maxParticles = cfg.maxParticles;
}

/** The tier the next effect would run at, given config + environment. */
export function detectFxTier(): FxTier {
  return detectTier(store());
}

/** Optional explicit host — mounts the shared canvas into the overlay mount
    (needed inside shadow roots so the canvas gets the bundle's styles). */
export function FxLayer(): JSX.Element {
  const mount = useOverlayMount();
  onMount(() => engine().ensureCanvas(mount));
  return null;
}

const FADE_MS = 120;

function wait(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

/* Elements currently hidden by an fx call → their prior inline visibility.
   Lets a later effect (e.g. materialize after explode) un-hide and re-snapshot
   the element instead of rasterizing a hidden (blank) one. */
const hiddenByFx = new WeakMap<HTMLElement, string>();

function hide(el: HTMLElement): () => void {
  if (!hiddenByFx.has(el)) hiddenByFx.set(el, el.style.visibility);
  el.style.visibility = 'hidden';
  return () => unhide(el);
}

function unhide(el: HTMLElement): void {
  const prev = hiddenByFx.get(el);
  hiddenByFx.delete(el);
  el.style.visibility = prev === 'hidden' ? '' : (prev ?? '');
}

function fade(el: HTMLElement, from: number, to: number): Promise<void> {
  if (typeof el.animate !== 'function') return Promise.resolve();
  return el
    .animate([{ opacity: from }, { opacity: to }], { duration: FADE_MS, easing: 'ease-out' })
    .finished.then(
      () => undefined,
      () => undefined,
    );
}

interface Prepared {
  tier: FxTier;
  budget: number;
  scale: number;
}

function prepare(el: HTMLElement, opts?: FxOptions): Prepared | undefined {
  if (typeof document === 'undefined' || !el.isConnected) return undefined;
  const s = store();
  const tier = detectTier(s);
  if (tier === 'off') return { tier, budget: 0, scale: 1 };
  engine().cancelFor(el);
  let budget = Math.min(tierBudget(tier, s), GLOBAL_BUDGET - engine().liveCount());
  if (opts?.maxParticles !== undefined) budget = Math.min(budget, opts.maxParticles);
  if (budget < 4) return { tier: 'off', budget: 0, scale: 1 };
  return { tier, budget, scale: tierDurationScale(tier) };
}

async function raster(el: HTMLElement, budget: number, opts?: FxOptions): Promise<PixelRaster> {
  const sampled = await rasterizeElement(el, budget);
  return sampled.kind === 'pixels' ? sampled : fallbackRaster(el, budget, opts?.colors);
}

/** Default particle size: fill the sampling grid (slight overlap kills
    hairline gaps) so the element shatters into contiguous fragments instead
    of a sparse dot field. */
function sizeFor(pixels: PixelRaster, opts?: FxOptions): number {
  return opts?.particleSize ?? Math.max(2, pixels.cell + 0.5);
}

async function explode(el: HTMLElement, opts?: FxOptions): Promise<void> {
  const prep = prepare(el, opts);
  if (!prep) return;
  if (prep.tier === 'off') {
    await fade(el, 1, 0);
    hide(el);
    return;
  }
  if (hiddenByFx.has(el)) unhide(el);
  const pixels = await raster(el, prep.budget, opts);
  hide(el); // end state is "destroyed": stays hidden, caller owns restore
  await engine().spawn({
    el,
    raster: pixels,
    phases: [{ kind: 'burst', dur: ((opts?.duration ?? 650) / 1000) * prep.scale }],
    gravity: opts?.gravity ?? 900,
    spread: opts?.spread ?? 1,
    size: sizeFor(pixels, opts),
  });
}

async function recreate(
  el: HTMLElement,
  opts?: FxOptions & { holdMs?: number; reappear?: 'converge' | 'fade' },
): Promise<void> {
  const prep = prepare(el, opts);
  if (!prep) return;
  if (prep.tier === 'off') {
    if (hiddenByFx.has(el)) unhide(el);
    await fade(el, 1, 0);
    const restore = hide(el);
    await wait(opts?.holdMs ?? 150);
    restore();
    await fade(el, 0, 1);
    return;
  }
  if (hiddenByFx.has(el)) unhide(el);
  const pixels = await raster(el, prep.budget, opts);
  const restore = hide(el);
  const burstDur = ((opts?.duration ?? 650) / 1000) * prep.scale;
  if (opts?.reappear === 'fade') {
    // Skip the converge: burst only, then the element itself fades back.
    // The engine's hold phase would re-light the already-faded burst
    // particles at 0.6 alpha, so the hold lives out here instead.
    await engine().spawn({
      el,
      raster: pixels,
      phases: [{ kind: 'burst', dur: burstDur }],
      gravity: opts?.gravity ?? 900,
      spread: opts?.spread ?? 1,
      size: sizeFor(pixels, opts),
    });
    await wait((opts?.holdMs ?? 150) * prep.scale);
    restore();
    await fade(el, 0, 1);
    return;
  }
  await engine().spawn({
    el,
    raster: pixels,
    phases: [
      { kind: 'burst', dur: burstDur },
      { kind: 'hold', dur: ((opts?.holdMs ?? 150) / 1000) * prep.scale },
      { kind: 'converge', dur: 0.6 * prep.scale },
    ],
    gravity: opts?.gravity ?? 900,
    spread: opts?.spread ?? 1,
    size: sizeFor(pixels, opts),
    onDone: restore,
  });
  await fade(el, 0, 1);
}

async function materialize(el: HTMLElement, opts?: FxOptions): Promise<void> {
  const prep = prepare(el, opts);
  if (!prep) return;
  if (prep.tier === 'off') {
    if (hiddenByFx.has(el)) unhide(el);
    await fade(el, 0, 1);
    return;
  }
  // Rasterize while visible (a hidden element snapshots blank), then hide.
  if (hiddenByFx.has(el)) unhide(el);
  const pixels = await raster(el, prep.budget, opts);
  const restore = hide(el);
  await engine().spawn({
    el,
    raster: pixels,
    phases: [{ kind: 'converge', dur: ((opts?.duration ?? 600) / 1000) * prep.scale }],
    gravity: 0,
    spread: opts?.spread ?? 1,
    size: sizeFor(pixels, opts),
    spawnRing: true,
    onDone: restore,
  });
  await fade(el, 0, 1);
}

function themePalette(el: HTMLElement, colors?: string[]): number[] {
  const inputs =
    colors?.length
      ? colors
      : (() => {
          const root = getComputedStyle(document.documentElement);
          return ['--accent', '--success', '--warning', '--info'].map((v) => root.getPropertyValue(v));
        })();
  const packed = inputs.map((c) => c && resolveColor(c.trim())).filter((c): c is number => typeof c === 'number');
  return packed.length ? packed : [0xffaaaaaa];
}

async function burst(el: HTMLElement, opts?: FxOptions & { particles?: number }): Promise<void> {
  const prep = prepare(el, opts);
  if (!prep || prep.tier === 'off') return; // celebratory only — safe no-op
  const rect = el.getBoundingClientRect();
  const palette = themePalette(el, opts?.colors);
  const n = Math.min(opts?.particles ?? (prep.tier === 'reduced' ? 40 : 120), prep.budget);
  const xs = new Float32Array(n);
  const ys = new Float32Array(n);
  const colors = new Uint32Array(n);
  for (let i = 0; i < n; i++) {
    xs[i] = rect.left + Math.random() * rect.width;
    ys[i] = rect.top + Math.random() * rect.height;
    colors[i] = palette[i % palette.length]!;
  }
  await engine().spawn({
    el,
    raster: { kind: 'pixels', xs, ys, colors, count: n, cell: 4 },
    phases: [{ kind: 'burst', dur: ((opts?.duration ?? 900) / 1000) * prep.scale }],
    gravity: opts?.gravity ?? 700,
    spread: (opts?.spread ?? 1) * 1.6,
    size: opts?.particleSize ?? 4,
  });
}

/** The shared FX system. All calls resolve when the effect finishes (or
    immediately when disabled) — callers never branch on capability. */
export const fx = {
  /** Element shatters and stays hidden (still in the DOM). */
  explode,
  /** Explode → hold → particles converge back → element restored.
      `reappear: 'fade'` swaps the converge for a plain fade-in. */
  recreate,
  /** Particles converge inward as the element appears. */
  materialize,
  /** Celebratory confetti from the element; the element is untouched. */
  burst,
  config: fxConfig,
};
