/* Capability / accessibility gating for particle effects.

   The tier is re-evaluated at every fx.* trigger so live changes to the
   user's reduced-motion preference (or a mid-session performance downgrade)
   take effect on the next call, not the next page load. The `degraded` flag
   lives in the shared global store so all @forge/ui copies agree. */

export type FxMode = 'auto' | 'full' | 'reduced' | 'off';
export type FxTier = 'full' | 'reduced' | 'off';

export interface FxGlobalConfig {
  mode: FxMode;
  /** Session-wide "this machine is struggling" flag, set by the engine's
      frame monitor. Caps auto tier at 'reduced'. */
  degraded: boolean;
  /** Hard per-effect particle cap applied on top of tier budgets. */
  maxParticles: number;
}

export const TIER_BUDGET: Record<Exclude<FxTier, 'off'>, number> = {
  full: 2500,
  reduced: 500,
};

/** Global particle cap across all concurrent effects. */
export const GLOBAL_BUDGET = 5000;

let reducedMotionMq: MediaQueryList | undefined;
let contrastMq: MediaQueryList | undefined;

function mq(query: string): MediaQueryList | undefined {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return undefined;
  return window.matchMedia(query);
}

export function detectFxTier(cfg: FxGlobalConfig): FxTier {
  if (cfg.mode === 'off') return 'off';
  if (cfg.mode === 'reduced') return 'reduced';
  if (cfg.mode === 'full') return 'full';

  // mode === 'auto': accessibility first, then hardware heuristics.
  reducedMotionMq ??= mq('(prefers-reduced-motion: reduce)');
  if (reducedMotionMq?.matches) return 'off';

  contrastMq ??= mq('(prefers-contrast: more)');
  let tier: FxTier = 'full';
  if (contrastMq?.matches) tier = 'reduced';

  if (typeof navigator !== 'undefined') {
    const cores = navigator.hardwareConcurrency;
    if (typeof cores === 'number' && cores > 0 && cores < 4) tier = 'reduced';
    const mem = (navigator as { deviceMemory?: number }).deviceMemory;
    if (typeof mem === 'number' && mem < 4) tier = 'reduced';
  }

  if (cfg.degraded) tier = 'reduced';
  return tier;
}

/** Per-effect particle budget for a tier ('off' gets none). */
export function tierBudget(tier: FxTier, cfg: FxGlobalConfig): number {
  if (tier === 'off') return 0;
  return Math.min(TIER_BUDGET[tier], cfg.maxParticles);
}

/** Duration scale for a tier — reduced motion runs shorter. */
export function tierDurationScale(tier: FxTier): number {
  return tier === 'reduced' ? 0.7 : 1;
}
