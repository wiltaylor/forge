/* Canvas particle engine — one fixed full-viewport canvas shared by all
   effects, rAF loop that only runs while jobs are live.

   Data is structure-of-arrays per job; render order is pre-sorted by packed
   color at spawn so fillStyle changes batch into runs (per-particle fade goes
   through globalAlpha, a number assignment, not a string). */

import type { PixelRaster } from './rasterize';

export type PhaseKind = 'burst' | 'hold' | 'converge';

export interface FxPhase {
  kind: PhaseKind;
  /** seconds */
  dur: number;
}

export interface SpawnParams {
  /** Anchor element — converge re-reads its rect each frame so particles
      land on the element even if it scrolled mid-effect. */
  el: HTMLElement;
  raster: PixelRaster;
  phases: FxPhase[];
  /** px/s² */
  gravity: number;
  /** initial velocity scale */
  spread: number;
  /** particle square size, CSS px */
  size: number;
  /** For materialize-style effects: spawn on a ring outside the element
      instead of at the home pixels. */
  spawnRing?: boolean;
  onDone?: () => void;
}

interface FxJob {
  el: HTMLElement;
  count: number;
  /** index stride for degraded rendering (1 = all, 2 = half) */
  step: number;
  xs: Float32Array;
  ys: Float32Array;
  vxs: Float32Array;
  vys: Float32Array;
  /** home offsets relative to the anchor rect origin at spawn */
  ox: Float32Array;
  oy: Float32Array;
  /** converge start positions, captured at phase flip */
  sxs: Float32Array | null;
  sys: Float32Array | null;
  /** per-particle life jitter (0.7–1) applied to burst fade */
  lifeScale: Float32Array;
  colors: Uint32Array;
  order: Uint32Array;
  size: number;
  gravity: number;
  phases: FxPhase[];
  phaseIdx: number;
  phaseT: number;
  done: boolean;
  resolve: () => void;
  onDone?: () => void;
}

const DRAG_PER_S = 2.2;
const MAX_DT = 0.05;
const SLOW_FRAME_S = 0.034;
const SLOW_FRAME_RUN = 20;

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

export class FxEngine {
  private canvas: HTMLCanvasElement | undefined;
  private ctx: CanvasRenderingContext2D | undefined;
  private jobs: FxJob[] = [];
  private byEl = new Map<HTMLElement, FxJob>();
  private raf = 0;
  private lastT = 0;
  private emaDt = 0;
  private slowRun = 0;
  private onDegrade: () => void;

  constructor(onDegrade: () => void) {
    this.onDegrade = onDegrade;
  }

  liveCount(): number {
    let n = 0;
    for (const j of this.jobs) n += Math.ceil(j.count / j.step);
    return n;
  }

  ensureCanvas(mount?: Node): void {
    if (this.canvas?.isConnected) return;
    const canvas = this.canvas ?? document.createElement('canvas');
    canvas.className = 'ffx-layer';
    (mount ?? document.body).appendChild(canvas);
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d') ?? undefined;
    this.resize();
  }

  private resize = (): void => {
    const c = this.canvas;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(window.innerWidth * dpr);
    c.height = Math.round(window.innerHeight * dpr);
    this.ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);
  };

  /** Cancel any running job on this element (restores nothing itself — the
      caller owns element visibility via onDone). */
  cancelFor(el: HTMLElement): void {
    const job = this.byEl.get(el);
    if (job && !job.done) this.finishJob(job);
  }

  spawn(params: SpawnParams): Promise<void> {
    this.ensureCanvas();
    return new Promise<void>((resolve) => {
      const { raster } = params;
      const n = raster.count;
      const rect = params.el.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const diag = Math.hypot(rect.width, rect.height);

      const job: FxJob = {
        el: params.el,
        count: n,
        step: 1,
        xs: new Float32Array(n),
        ys: new Float32Array(n),
        vxs: new Float32Array(n),
        vys: new Float32Array(n),
        ox: new Float32Array(n),
        oy: new Float32Array(n),
        sxs: null,
        sys: null,
        lifeScale: new Float32Array(n),
        colors: raster.colors.slice(0, n),
        order: new Uint32Array(n),
        size: params.size,
        gravity: params.gravity,
        phases: params.phases,
        phaseIdx: 0,
        phaseT: 0,
        done: false,
        resolve,
        onDone: params.onDone,
      };

      for (let i = 0; i < n; i++) {
        const hx = raster.xs[i]!;
        const hy = raster.ys[i]!;
        job.ox[i] = hx - rect.left;
        job.oy[i] = hy - rect.top;
        job.lifeScale[i] = 0.7 + Math.random() * 0.3;
        if (params.spawnRing) {
          // Materialize: start scattered on a ring just outside the element.
          const a = Math.random() * Math.PI * 2;
          const r = diag * (0.45 + Math.random() * 0.35);
          job.xs[i] = cx + Math.cos(a) * r;
          job.ys[i] = cy + Math.sin(a) * r;
        } else {
          job.xs[i] = hx;
          job.ys[i] = hy;
          const dx = hx - cx;
          const dy = hy - cy;
          const d = Math.hypot(dx, dy) || 1;
          const speed = (35 + Math.random() * 110 + d * 1.6) * params.spread;
          job.xs[i] = hx;
          job.ys[i] = hy;
          job.vxs[i] = (dx / d) * speed + (Math.random() - 0.5) * 45 * params.spread;
          job.vys[i] = (dy / d) * speed + (Math.random() - 0.5) * 45 * params.spread - 25 * params.spread;
        }
        job.order[i] = i;
      }
      // Sort render order by color so fillStyle changes batch into runs.
      const colors = job.colors;
      Array.prototype.sort.call(job.order, (a: number, b: number) => colors[a]! - colors[b]!);

      if (job.phases[0]?.kind === 'converge') this.beginConverge(job);

      this.jobs.push(job);
      this.byEl.set(params.el, job);
      this.start();
    });
  }

  private beginConverge(job: FxJob): void {
    job.sxs = job.xs.slice();
    job.sys = job.ys.slice();
  }

  private finishJob(job: FxJob): void {
    job.done = true;
    if (this.byEl.get(job.el) === job) this.byEl.delete(job.el);
    job.onDone?.();
    job.resolve();
  }

  private start(): void {
    if (this.raf) return;
    this.lastT = performance.now();
    this.emaDt = 0;
    this.slowRun = 0;
    window.addEventListener('resize', this.resize);
    const loop = (now: number) => {
      const dt = Math.min(MAX_DT, (now - this.lastT) / 1000);
      this.lastT = now;
      this.monitor(now, dt);
      this.tick(dt);
      this.render();
      if (this.jobs.length) {
        this.raf = requestAnimationFrame(loop);
      } else {
        this.stop();
      }
    };
    this.raf = requestAnimationFrame(loop);
  }

  private stop(): void {
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
    window.removeEventListener('resize', this.resize);
    if (this.ctx && this.canvas) this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  private monitor(_now: number, dt: number): void {
    this.emaDt = this.emaDt === 0 ? dt : this.emaDt * 0.8 + dt * 0.2;
    if (this.emaDt > SLOW_FRAME_S) {
      if (++this.slowRun >= SLOW_FRAME_RUN) {
        this.slowRun = 0;
        for (const job of this.jobs) if (job.step === 1) job.step = 2;
        this.onDegrade();
      }
    } else {
      this.slowRun = 0;
    }
  }

  private tick(dt: number): void {
    const drag = Math.exp(-DRAG_PER_S * dt);
    for (const job of this.jobs) {
      job.phaseT += dt;
      let phase = job.phases[job.phaseIdx];
      while (phase && job.phaseT >= phase.dur) {
        job.phaseT -= phase.dur;
        job.phaseIdx++;
        phase = job.phases[job.phaseIdx];
        if (phase?.kind === 'converge') this.beginConverge(job);
      }
      if (!phase) {
        this.finishJob(job);
        continue;
      }

      if (phase.kind === 'burst') {
        const g = job.gravity * dt;
        for (let i = 0; i < job.count; i += job.step) {
          const vx = job.vxs[i]! * drag;
          const vy = (job.vys[i]! + g) * drag;
          job.vxs[i] = vx;
          job.vys[i] = vy;
          job.xs[i] = job.xs[i]! + vx * dt;
          job.ys[i] = job.ys[i]! + vy * dt;
        }
      } else if (phase.kind === 'converge' && job.sxs && job.sys) {
        // Re-anchor homes each frame: one rect read per job.
        const rect = job.el.getBoundingClientRect();
        const t = easeInOutCubic(Math.min(1, job.phaseT / phase.dur));
        for (let i = 0; i < job.count; i += job.step) {
          const hx = rect.left + job.ox[i]!;
          const hy = rect.top + job.oy[i]!;
          const sx = job.sxs[i]!;
          const sy = job.sys[i]!;
          job.xs[i] = sx + (hx - sx) * t;
          job.ys[i] = sy + (hy - sy) * t;
        }
      }
      // 'hold': particles float in place.
    }
    this.jobs = this.jobs.filter((j) => !j.done);
  }

  private render(): void {
    const ctx = this.ctx;
    if (!ctx || !this.canvas) return;
    ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);

    for (const job of this.jobs) {
      const phase = job.phases[job.phaseIdx];
      if (!phase) continue;
      const t = Math.min(1, job.phaseT / phase.dur);
      const size = job.size;
      const half = size / 2;
      let lastColor = -1;
      for (let k = 0; k < job.count; k++) {
        const i = job.order[k]!;
        if (job.step === 2 && i % 2 === 1) continue;
        const packed = job.colors[i]!;
        // Burst: fade out over the last 40% of (jittered) life.
        // Converge: fade back in over the first 30%.
        let fade = 1;
        if (phase.kind === 'burst') {
          const lt = t / job.lifeScale[i]!;
          fade = lt >= 1 ? 0 : lt < 0.6 ? 1 : 1 - (lt - 0.6) / 0.4;
        } else if (phase.kind === 'converge') {
          fade = Math.min(1, 0.35 + t);
        } else {
          fade = 0.6;
        }
        if (fade <= 0) continue;
        if (packed !== lastColor) {
          lastColor = packed;
          ctx.fillStyle = `rgb(${packed & 0xff},${(packed >> 8) & 0xff},${(packed >> 16) & 0xff})`;
        }
        ctx.globalAlpha = (((packed >>> 24) & 0xff) / 255) * fade;
        ctx.fillRect(job.xs[i]! - half, job.ys[i]! - half, size, size);
      }
    }
    ctx.globalAlpha = 1;
  }
}
