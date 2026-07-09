/* Categorical series colours use CHART_SERIES — a fixed, validated order
   (see "Chart colours" in reference/tokens.md; never cycle past it, fold
   extras into "Other" = --fg-2). Semantic data should pass tone: props
   instead; status hues never impersonate a series. */

/** Semantic tone a datum/series may carry instead of a positional series colour. */
export type ChartTone = 'accent' | 'danger' | 'success' | 'warning' | 'info';

/* Fixed categorical order — validated with the dataviz palette checker:
   min adjacent CVD ΔE 17.8 (dark) / 16.9 (light). Never reorder or cycle. */
export const CHART_SERIES: string[] = [
  'var(--accent)', 'var(--danger)', 'var(--success)', 'var(--warning)', 'var(--info)',
];
export const CHART_SERIES_BG: string[] = [
  'var(--accent-bg)', 'var(--danger-bg)', 'var(--success-bg)', 'var(--warning-bg)', 'var(--info-bg)',
];
export const CHART_OTHER = 'var(--fg-2)';  /* the "Other" fold — not a series slot */

export function seriesColor(i: number, tone?: ChartTone): string {
  if (tone) return `var(--${tone})`;
  return i < CHART_SERIES.length ? CHART_SERIES[i]! : CHART_OTHER;
}
export function seriesBg(i: number, tone?: ChartTone): string {
  if (tone) return `var(--${tone}-bg)`;
  return i < CHART_SERIES_BG.length ? CHART_SERIES_BG[i]! : 'color-mix(in oklab, var(--fg-2) 14%, transparent)';
}

/* 1/2/5-step "nice" ticks from 0 (charts here are zero-based magnitudes). */
export function niceTicks(max: number, n = 4): number[] {
  if (!(max > 0)) return [0, 1];
  const raw = max / n;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((s) => s * mag).find((s) => s >= raw)!;
  const ticks: number[] = [];
  for (let v = 0; v <= max + step * 0.001; v += step) ticks.push(v);
  if (ticks[ticks.length - 1]! < max) ticks.push(ticks[ticks.length - 1]! + step);
  return ticks;
}
