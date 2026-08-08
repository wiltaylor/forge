/**
 * Emit forge-egui's geometry, type, motion and control-height tokens.
 *
 * These are the tokens a pixel canvas can express and a terminal cannot, so
 * forge-tui takes none of them and this generator has one kit to serve. Every
 * value the kit holds is a length in pixels or a duration in milliseconds, and
 * egui wants points and seconds — those two conversions are most of the job.
 * The spacing ramp is the derivation beyond them: the kit holds its first step
 * and multiplies, so `spacingRamp` holds the source to that.
 *
 * The layout below — which token fills which struct field, and the prose each
 * struct carries — is the only thing this module knows that the source does
 * not. It is a fact about forge-egui's types, so it lives beside the code that
 * writes them, exactly as the palette layout does in `./rust-palette.mjs`.
 */
import { SOURCE_PATH, isSchemeToken, tokenNamed, tokens } from '../../packages/tokens/tokens.source.mjs';
import { f32, moduleDoc } from './rust.mjs';

const KIT = 'egui';

/**
 * Each token struct: its Rust type, its prose, and the token behind each field.
 *
 * `prefix` marks a struct whose fields are named after their tokens — the field
 * is the token name without that prefix. `Space` and `MotionDurations` name a
 * role instead, because the source numbers those tokens and a Rust field cannot
 * be called `1`.
 */
export const STRUCTS = [
  {
    type: 'Radius',
    doc: ['Corner radii. A pill is `height / 2.0` at the call site, not a token.'],
    unit: 'points',
    prefix: 'r-',
    fields: [
      ['sm', 'r-sm'],
      ['md', 'r-md'],
      ['lg', 'r-lg'],
    ],
  },
  {
    type: 'Space',
    doc: [
      'The spacing scale, held as its base step. `space.x(n)` is n steps,',
      'which is how the rest of the `--sp-*` ramp is reached — the generator',
      'checks that the ramp really is its index times this step.',
    ],
    unit: 'points',
    fields: [['base', 'sp-1']],
  },
  {
    type: 'TypeScale',
    doc: [
      'The type scale. `xs`..`lg` are the body sizes; `xl`, `xl2` and `xl3`',
      'are the heading sizes.',
    ],
    unit: 'points',
    prefix: 'fs-',
    fields: [
      ['xs', 'fs-xs'],
      ['sm', 'fs-sm'],
      ['base', 'fs-base'],
      ['md', 'fs-md'],
      ['lg', 'fs-lg'],
      ['xl', 'fs-xl'],
      ['xl2', 'fs-2xl'],
      ['xl3', 'fs-3xl'],
    ],
  },
  {
    type: 'ControlHeights',
    doc: ['Control heights — the height a button, input or select stands at.'],
    unit: 'points',
    prefix: 'h-',
    fields: [
      ['sm', 'h-sm'],
      ['md', 'h-md'],
      ['lg', 'h-lg'],
      ['xl', 'h-xl'],
    ],
  },
  {
    type: 'MotionDurations',
    doc: ['Motion durations, in seconds. The source authors milliseconds.'],
    unit: 'seconds',
    fields: [
      ['fast', 'dur-1'],
      ['base', 'dur-2'],
      ['slow', 'dur-3'],
    ],
  },
];

/** The shell dimensions, which are free constants rather than a token struct. */
export const CONSTANTS = [
  ['SIDEBAR_WIDTH', 'sidebar-w'],
  ['SIDEBAR_RAIL', 'sidebar-rail-w'],
  ['TOPBAR_HEIGHT', 'topbar-h'],
  ['STATUSBAR_HEIGHT', 'statusbar-h'],
];

/** What the source authors a field's value in, and what egui wants it multiplied by. */
const UNITS = {
  points: { authoredIn: 'px', scale: 1 },
  seconds: { authoredIn: 'ms', scale: 1 / 1000 },
};

/**
 * The number a token's authored value becomes, in the unit the field holds.
 *
 * A token that is not the length or duration the layout expects is a mistake in
 * one of the two files, and either way the kit would take a number that means
 * something else. Refuse here, where the message can name both.
 *
 * @param {string} name the token name.
 * @param {'points'|'seconds'} unit what the field holds.
 * @returns {number} the value in that unit.
 */
export function measure(name, unit) {
  const { authoredIn, scale } = UNITS[unit];
  const token = tokenNamed(name, KIT);
  if (isSchemeToken(token)) {
    throw new Error(`--${name} is authored per scheme, and forge-egui's geometry holds one value`);
  }
  const match = String(token.value.raw).match(new RegExp(`^(-?\\d+(?:\\.\\d+)?)${authoredIn}$`));
  if (!match) {
    throw new Error(
      `--${name} is "${token.value.raw}" in ${SOURCE_PATH}, and forge-egui reads it as ${unit}. ` +
        `Author it in ${authoredIn}.`,
    );
  }
  return Number(match[1]) * scale;
}

/**
 * Check that the `--sp-*` ramp really is its index times its first step.
 *
 * forge-egui holds the whole spacing ramp as that one step and multiplies:
 * `space.x(6.0)` is what `--sp-6` declares. That is a derivation rule, so it
 * belongs here rather than in the kit's prose. Without it the nine steps the
 * struct does not name go back to being a silent transcription — author
 * `--sp-6` as 25px and egui would keep painting 24 with the check still green.
 *
 * @returns {number} the base step, in points.
 */
export function spacingRamp() {
  const base = measure('sp-1', 'points');
  for (const { name } of tokens.filter((token) => /^sp-\d+$/.test(token.name))) {
    const steps = Number(name.slice('sp-'.length));
    const wanted = steps * base;
    const authored = measure(name, 'points');
    if (authored !== wanted) {
      throw new Error(
        `--${name} is ${authored}px in ${SOURCE_PATH}, and forge-egui reaches it as ${steps} × ` +
          `--sp-1 (${wanted}px). Either restore the ramp or give the kit the step of its own.`,
      );
    }
  }
  return base;
}

/** The rustdoc line a field or constant carries: its token, and the note if it has one. */
function provenance(name) {
  const { note } = tokenNamed(name, KIT);
  return `/// \`--${name}\`${note ? ` — ${note}.` : ''}`;
}

/**
 * rustfmt's `struct_lit_width`: a struct literal whose fields fit in this many
 * columns goes on one line. The emitted text matches what `cargo fmt` wants, so
 * that formatting the crate never fights the generator.
 */
const STRUCT_LIT_WIDTH = 18;

/** One token struct: the type, then the `Default` impl holding the source's values. */
function struct({ type, doc, unit, fields }) {
  const lines = [
    ...doc.map((line) => `/// ${line}`),
    '#[derive(Clone, Copy, Debug, PartialEq)]',
    `pub struct ${type} {`,
  ];
  for (const [field, name] of fields) {
    lines.push(`    ${provenance(name)}`, `    pub ${field}: f32,`);
  }

  const assignments = fields.map(([field, name]) => `${field}: ${f32(measure(name, unit))}`);
  const oneLine = assignments.join(', ');
  const literal =
    oneLine.length <= STRUCT_LIT_WIDTH
      ? [`        ${type} { ${oneLine} }`]
      : [`        ${type} {`, ...assignments.map((a) => `            ${a},`), '        }'];

  lines.push('}', '', `impl Default for ${type} {`, '    fn default() -> Self {', ...literal, '    }', '}');
  return lines;
}

/** @returns {string} the full text of `crates/forge-egui/src/theme/tokens.rs`. */
export function renderEguiTokens() {
  // `Space` names one step and the kit multiplies for the rest, so the ramp
  // behind it has to hold before any of it is worth emitting.
  spacingRamp();

  const lines = [
    ...moduleDoc([
      'Geometry, type, motion and control-height tokens.',
      '',
      'These are the tokens a pixel canvas can express. A terminal cannot, so',
      'forge-tui has no counterpart of this file. Lengths are egui points, one',
      'per pixel of the token source; durations are seconds.',
      '',
      'Each field names the token it carries. Where a token name cannot be a',
      'Rust identifier the field reverses it: `--fs-2xl` is `xl2`.',
    ]),
  ];

  for (const spec of STRUCTS) lines.push('', ...struct(spec));

  lines.push(
    '',
    '// Shell dimensions. The rail and the status bar are scoped to this kit in',
    '// the token source: the web shell has no equivalent of either.',
  );
  for (const [constant, name] of CONSTANTS) {
    lines.push('', provenance(name), `pub const ${constant}: f32 = ${f32(measure(name, 'points'))};`);
  }

  return `${lines.join('\n')}\n`;
}
