/**
 * Emit both Rust kits' palettes from the token source.
 *
 * The two kits hold the same token set in the same struct layout and differ
 * only in how a translucent tint reaches them. A terminal has no alpha channel,
 * so forge-tui takes each tint already composited over the surface the source
 * names. egui paints truecolor, so forge-egui takes the tint's own colour with
 * the alpha quantised to a byte. Both derivations run in `./oklch.mjs`, so the
 * two kits cannot disagree about what a tint is.
 *
 * The layout below — which token fills which struct field — is the only thing
 * this module knows that the source does not. It is a fact about the Rust
 * `Theme` struct, so it lives beside the code that writes Rust.
 */
import { SOURCE_PATH, tokenNamed, valueFor } from '../../packages/tokens/tokens.source.mjs';
import { bannerLines } from './banner.mjs';
import { alphaByte, flatten, toRgb } from './oklch.mjs';
import { formatValue } from './tokens-css.mjs';

/** An sRGB triple as the `0xRRGGBB` literal both kits' `rgb()` takes. */
export const hexLiteral = (rgb) =>
  `0x${rgb.map((v) => v.toString(16).toUpperCase().padStart(2, '0')).join('')}`;

/** The two index ramps, in the order the struct declares them. */
const RAMPS = [
  { field: 'bg', names: ['bg-0', 'bg-1', 'bg-2', 'bg-3', 'bg-4'] },
  { field: 'fg', names: ['fg-0', 'fg-1', 'fg-2', 'fg-3'] },
];

/** Each nested struct, and the token that fills each of its fields. */
const STRUCTS = [
  {
    field: 'border',
    type: 'BorderTokens',
    fields: [
      ['subtle', 'border-subtle'],
      ['default', 'border'],
      ['strong', 'border-strong'],
    ],
  },
  {
    field: 'accent',
    type: 'Accent',
    fields: [
      ['base', 'accent'],
      ['hover', 'accent-hover'],
      ['press', 'accent-press'],
      ['bg', 'accent-bg'],
      ['fg', 'accent-fg'],
      ['contrast', 'accent-contrast'],
    ],
  },
  ...['success', 'warning', 'danger', 'info'].map((tone) => ({
    field: tone,
    type: 'SemanticTriple',
    fields: [
      ['base', tone],
      ['bg', `${tone}-bg`],
      ['fg', `${tone}-fg`],
    ],
  })),
];

const SCHEMES = [
  { scheme: 'dark', name: 'forge-dark', variant: 'Dark' },
  { scheme: 'light', name: 'forge-light', variant: 'Light' },
];

/** Every token the palettes read, in emission order. */
const paletteTokens = () => [
  ...RAMPS.flatMap((ramp) => ramp.names),
  ...STRUCTS.flatMap((struct) => struct.fields.map(([, token]) => token)),
];

/** True when the token's value in this scheme is a translucent tint. */
const isTint = (value) => value.alpha !== undefined;

/** The name of the byte constant forge-egui stores this alpha in. */
const alphaConstant = (alpha) => `A${Math.round(alpha * 100)}`;

/**
 * Pair each alpha with the constant that will hold it.
 *
 * The name states the alpha as a percentage, so two alphas a fraction of a
 * percent apart would both claim it. That emits two `const` items of one name
 * and the crate stops compiling, naming a Rust file nobody edited. Refuse here
 * instead, where the message can name the source.
 *
 * @param {number[]} alphas every distinct tint alpha.
 * @returns {[string, number][]} the constant name and value of each, in order.
 */
export function alphaConstants(alphas) {
  const claimed = new Map();
  for (const alpha of alphas) {
    const name = alphaConstant(alpha);
    const holder = claimed.get(name);
    if (holder !== undefined) {
      throw new Error(
        `the tint alphas ${holder} and ${alpha} both want the constant ${name}. ` +
          `Two alphas in ${SOURCE_PATH} round to the same percentage.`,
      );
    }
    claimed.set(name, alpha);
  }
  return [...claimed].map(([name, alpha]) => [name, alpha]);
}

/**
 * The trailing comment for a field.
 *
 * A converted colour states the expression the source authored, so a reader can
 * check the conversion against the stylesheet without leaving the file. It is
 * rendered by the CSS generator's own formatter, so the two cannot drift apart.
 * A colour the source wrote as a literal states its note instead, if it has one.
 */
function annotation(token, value, { namesOverSurface }) {
  if (isTint(value)) {
    const expression = formatValue(value);
    return namesOverSurface ? `${expression} over ${value.over}` : expression;
  }
  if (token.note) return token.note;
  if (value.oklch !== undefined) return formatValue(value);
  return null;
}

/** Lay out `code // comment` rows, comments aligned one space past the widest row. */
function aligned(rows, indent) {
  const width = Math.max(0, ...rows.filter((row) => row.comment).map((row) => row.code.length));
  return rows.map((row) =>
    row.comment ? `${indent}${row.code.padEnd(width)} // ${row.comment}` : `${indent}${row.code}`,
  );
}

/**
 * The body of one `Theme` literal, from `name:` to the last semantic triple.
 *
 * `kit` supplies the two things the kits differ by: how a value becomes a Rust
 * expression, and whether a tint's comment names the surface it flattened over.
 */
function themeBody(kit, { scheme, name, variant }, indent) {
  const inner = `${indent}    `;
  const cell = (tokenName) => {
    const token = tokenNamed(tokenName);
    const value = valueFor(token, scheme);
    return { value, comment: annotation(token, value, kit), expression: kit.expression(value, scheme) };
  };

  const lines = [`${indent}name: "${name}",`, `${indent}scheme: Scheme::${variant},`];

  for (const ramp of RAMPS) {
    const rows = ramp.names.map((tokenName) => {
      const { expression, comment } = cell(tokenName);
      return { code: `${expression},`, comment };
    });
    lines.push(`${indent}${ramp.field}: [`, ...aligned(rows, inner), `${indent}],`);
  }

  for (const struct of STRUCTS) {
    const rows = struct.fields.map(([field, tokenName]) => {
      const { expression, comment } = cell(tokenName);
      return { code: `${field}: ${expression},`, comment };
    });
    lines.push(`${indent}${struct.field}: ${struct.type} {`, ...aligned(rows, inner), `${indent}},`);
  }

  return lines;
}

/** The `//!` header: the generated-file banner, then the module's own prose. */
const moduleDoc = (prose) => [...bannerLines(), '', ...prose].map((line) => (line ? `//! ${line}` : '//!'));

/* ------------------------------------------------------------------ forge-tui */

const TUI = {
  namesOverSurface: true,
  /** Every colour is opaque: a tint arrives already composited over its surface. */
  expression(value, scheme) {
    if (!isTint(value)) return `rgb(${hexLiteral(toRgb(value))})`;
    const surface = toRgb(valueFor(tokenNamed(value.over), scheme));
    return `rgb(${hexLiteral(flatten(toRgb(value), surface, value.alpha))})`;
  },
};

/** @returns {string} the full text of `crates/forge-tui/src/theme/palette.rs`. */
export function renderTuiPalette() {
  const lines = [
    ...moduleDoc([
      'Forge token palette as compile-time constants.',
      '',
      'The neutral ramps are the sRGB literals the source authors. The accent',
      'and semantic tokens are authored in OKLCH. The generator converts them,',
      'and each states the expression it came from.',
      '',
      'Note that `packages/term/src/theme.ts` carries `#5A8FDB` as the accent',
      '*fallback*. That is a hand-picked stand-in for when CSS resolution',
      'fails, and deliberately not the token value. These values are what',
      'browsers actually paint.',
      '',
      'Terminals have no alpha. Each `*-bg` tint thus arrives pre-composited',
      'over the surface its token names — the card it usually sits on. A tint',
      'painted on another surface will be marginally off. Call',
      '[`blend`](super::color::blend) directly if that matters.',
    ]),
    '',
    'use super::color::rgb;',
    'use super::{Accent, BorderTokens, Scheme, SemanticTriple, Theme};',
  ];

  for (const scheme of SCHEMES) {
    lines.push(
      '',
      `pub const ${scheme.scheme.toUpperCase()}: Theme = Theme {`,
      ...themeBody(TUI, scheme, '    '),
      '};',
    );
  }
  return `${lines.join('\n')}\n`;
}

/* ----------------------------------------------------------------- forge-egui */

const EGUI = {
  namesOverSurface: false,
  /** A tint keeps its own colour and carries the alpha as a byte. */
  expression(value) {
    const literal = `rgb(${hexLiteral(toRgb(value))})`;
    return isTint(value) ? `with_alpha(${literal}, ${alphaConstant(value.alpha)})` : literal;
  },
};

/**
 * Geometry, type, motion and control heights still come from the kit's own
 * defaults. Issue #14 covers generating them.
 */
const EGUI_DEFAULTED = ['radius', 'space', 'type_scale', 'control', 'motion'];

/** Every distinct tint alpha the palettes use, ascending. */
function tintAlphas() {
  const alphas = new Set();
  for (const tokenName of paletteTokens()) {
    for (const { scheme } of SCHEMES) {
      const value = valueFor(tokenNamed(tokenName), scheme);
      if (isTint(value)) alphas.add(value.alpha);
    }
  }
  return [...alphas].sort((a, b) => a - b);
}

/** @returns {string} the full text of `crates/forge-egui/src/theme/palette.rs`. */
export function renderEguiPalette() {
  const lines = [
    ...moduleDoc([
      'Forge token palette.',
      '',
      'The neutral ramps are the sRGB literals the source authors. The accent',
      'and semantic tokens are authored in OKLCH. The generator converts them,',
      'and each states the expression it came from.',
      '',
      'forge-tui pre-composites its translucent `*-bg` tints over the card',
      'surface, because a terminal has no alpha channel. These tints instead',
      'carry REAL alpha, exactly like the web. Thus they composite correctly',
      'over any surface. Both kits derive their tints from the same source',
      'entry, so the two cannot disagree about what a tint is.',
    ]),
    '',
    'use super::color::{rgb, with_alpha};',
    'use super::{Accent, BorderTokens, Scheme, SemanticTriple, Theme};',
  ];

  lines.push('');
  for (const [name, alpha] of alphaConstants(tintAlphas())) {
    lines.push(
      `/// ${Math.round(alpha * 100)}% alpha, quantised to a byte.`,
      `const ${name}: u8 = ${alphaByte(alpha)};`,
    );
  }

  for (const scheme of SCHEMES) {
    lines.push(
      '',
      `pub fn ${scheme.scheme}() -> Theme {`,
      '    Theme {',
      ...themeBody(EGUI, scheme, '        '),
      ...EGUI_DEFAULTED.map((field) => `        ${field}: Default::default(),`),
      '    }',
      '}',
    );
  }
  return `${lines.join('\n')}\n`;
}
