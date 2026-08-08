/**
 * Colour conversion — OKLCH to sRGB, and the two rules a kit derives a tint by.
 *
 * This is the only place in the repo that converts a colour. Every palette
 * reaches sRGB through here, so no two kits can round a token differently and
 * nobody performs a colour-space conversion by hand again.
 *
 * Gamut
 * -----
 * Several authored colours sit outside sRGB. Clipping each channel on its own
 * turns the hue, because the three channels clip by different amounts. So the
 * chroma gives way instead: the lightness and the hue the author chose hold,
 * and C reduces to the largest value that still fits. That rule reproduces
 * every colour in the committed palettes.
 */

/** Linear-light sRGB from OKLab (Ottosson's matrices). */
function oklabToLinearSrgb(lightness, a, b) {
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

/** Linear-light channel to the sRGB transfer curve, both 0–1. */
const encode = (u) => (u <= 0.0031308 ? 12.92 * u : 1.055 * u ** (1 / 2.4) - 0.055);

/** Linear-light sRGB for a polar OKLCH colour. */
function linearSrgbFor(lightness, chroma, hue) {
  const radians = (hue * Math.PI) / 180;
  return oklabToLinearSrgb(lightness, chroma * Math.cos(radians), chroma * Math.sin(radians));
}

/**
 * Rounding slack. A channel that the arithmetic puts a hair outside 0 or 1 is
 * still in the gamut.
 */
const TOLERANCE = 1e-9;

const fitsSrgb = (lightness, chroma, hue) =>
  linearSrgbFor(lightness, chroma, hue).every((u) => u >= -TOLERANCE && u <= 1 + TOLERANCE);

/** Bisections of the chroma range — far past the point the 8-bit result stops moving. */
const BISECTIONS = 64;

/** The most chroma this lightness and hue can carry inside sRGB, never above `chroma`. */
function chromaInGamut(lightness, chroma, hue) {
  let low = 0;
  let high = chroma;
  for (let i = 0; i < BISECTIONS; i += 1) {
    const mid = (low + high) / 2;
    if (fitsSrgb(lightness, mid, hue)) low = mid;
    else high = mid;
  }
  return low;
}

const toByte = (u) => Math.round(Math.min(1, Math.max(0, encode(u))) * 255);

/** @returns {number[]} the sRGB bytes of an `[L, C, H]` colour, fitted to the gamut. */
export function oklchToRgb([lightness, chroma, hue]) {
  const fitted = fitsSrgb(lightness, chroma, hue)
    ? chroma
    : chromaInGamut(lightness, chroma, hue);
  return linearSrgbFor(lightness, fitted, hue).map(toByte);
}

/** @returns {number[]} the sRGB bytes of a `#RRGGBB` literal. */
export function hexToRgb(hex) {
  const digits = hex.replace('#', '');
  if (!/^[0-9a-fA-F]{6}$/.test(digits)) throw new Error(`not a #RRGGBB literal: ${hex}`);
  return [0, 2, 4].map((i) => parseInt(digits.slice(i, i + 2), 16));
}

/** @returns {number[]} the sRGB bytes of an authored `{ hex }` or `{ oklch }` value. */
export function toRgb(value) {
  if (value.hex !== undefined) return hexToRgb(value.hex);
  if (value.oklch !== undefined) return oklchToRgb(value.oklch);
  throw new Error(`not a colour: ${JSON.stringify(value)}`);
}

/**
 * sRGB composite of `fg` at `alpha` over `bg` — what a browser paints, and what
 * both kits' `blend` computes. This is how a target with no alpha channel gets
 * a translucent tint.
 */
export const flatten = (fg, bg, alpha) =>
  fg.map((channel, i) => Math.round(channel * alpha + bg[i] * (1 - alpha)));

/** The web's fractional alpha as the byte a truecolor target stores it in. */
export const alphaByte = (alpha) => Math.round(alpha * 255);
