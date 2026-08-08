/**
 * Emit the typed TypeScript theme from the token source.
 *
 * `packages/tokens/src/theme.gen.ts` is the token set as a TypeScript value:
 * the `Theme` type, the two built-in ramps, and the function that flattens a
 * theme into custom properties. The engine around it — `defineTheme` and
 * `applyTheme` — is behaviour rather than values, so it stays hand-written in
 * `packages/tokens/src/theme.ts` and re-exports what this file emits.
 *
 * The theme carries every token the web kit declares, not the colours alone.
 * A declared token that the layout below does not name is an error here, so
 * geometry, spacing, type scale, control heights, durations and easings cannot
 * go back to being reachable only through the untyped `vars` escape hatch.
 *
 * The layout — which token fills which field, and the prose each type carries —
 * is the only thing this module knows that the source does not. It is a fact
 * about the TypeScript API, so it lives beside the code that writes it, exactly
 * as the Rust layouts do in `./rust-palette.mjs` and `./egui-tokens.mjs`.
 */
import { SOURCE_PATH, inKit, tokenNamed, tokens, valueFor } from '../../packages/tokens/tokens.source.mjs';
import { bannerLines } from './banner.mjs';
import { formatValue } from './tokens-css.mjs';

/** The kit the stylesheet and the typed theme share. */
const KIT = 'web';

/** The path this generator writes, for the message a layout hole fails with. */
const LAYOUT_PATH = 'scripts/generate/theme-ts.mjs';

/**
 * The two index ramps. They stay tuples: `bg[2]` is the hover surface, and the
 * position carries that meaning in a way a name would only repeat.
 */
export const RAMPS = [
  { field: 'bg', doc: 'Backgrounds', names: ['bg-0', 'bg-1', 'bg-2', 'bg-3', 'bg-4'] },
  { field: 'fg', doc: 'Foregrounds', names: ['fg-0', 'fg-1', 'fg-2', 'fg-3'] },
];

/** A semantic tone. The four share one type, so they share its field prose. */
const tone = (name, doc) => ({
  field: name,
  type: 'SemanticTriple',
  typeDoc: ['One semantic tone: the colour itself, the tint it sits on, and the text on that tint.'],
  doc,
  fields: [
    ['base', name, 'Solid tone colour (borders, icons, strokes).'],
    ['bg', `${name}-bg`, 'Translucent tint used as a background.'],
    ['fg', `${name}-fg`, 'Text colour readable on the tint.'],
  ],
});

/**
 * Each group of the theme: its field, its type, and the token behind each of
 * the type's fields.
 *
 * `prefix` marks a group whose fields are named after their tokens — the field
 * is the token name without that prefix, camel-cased. A group with no `prefix`
 * names a role instead, because its tokens have no common stem: `--border` is
 * `border.default`, and `--accent` is `accent.base`.
 *
 * A field entry is `[field, token]`, or `[field, token, doc]` where the type is
 * shared and the prose has to describe the role rather than the one token.
 */
export const GROUPS = [
  {
    field: 'border',
    type: 'ThemeBorder',
    typeDoc: ['Border strengths, subtle → strong.'],
    doc: 'Borders.',
    fields: [
      ['subtle', 'border-subtle'],
      ['default', 'border'],
      ['strong', 'border-strong'],
    ],
  },
  {
    field: 'accent',
    type: 'ThemeAccent',
    typeDoc: ['The accent colour and the states it takes.'],
    doc: 'The accent colour.',
    fields: [
      ['base', 'accent'],
      ['hover', 'accent-hover'],
      ['press', 'accent-press'],
      ['bg', 'accent-bg'],
      ['fg', 'accent-fg'],
      ['contrast', 'accent-contrast'],
    ],
  },
  tone('success', 'The success tone.'),
  tone('warning', 'The warning tone.'),
  tone('danger', 'The danger tone.'),
  tone('info', 'The info tone.'),
  {
    field: 'fonts',
    type: 'ThemeFonts',
    typeDoc: ['The two font stacks.'],
    doc: 'Font stacks.',
    prefix: 'font-',
    fields: [
      ['sans', 'font-sans'],
      ['mono', 'font-mono'],
    ],
  },
  {
    field: 'fontSize',
    type: 'ThemeFontSize',
    typeDoc: ['The type scale. `xs`..`lg` are the body sizes; `xl`..`3xl` are the headings.'],
    doc: 'The type scale.',
    prefix: 'fs-',
    fields: [
      ['xs', 'fs-xs'],
      ['sm', 'fs-sm'],
      ['base', 'fs-base'],
      ['md', 'fs-md'],
      ['lg', 'fs-lg'],
      ['xl', 'fs-xl'],
      ['2xl', 'fs-2xl'],
      ['3xl', 'fs-3xl'],
    ],
  },
  {
    field: 'lineHeight',
    type: 'ThemeLineHeight',
    typeDoc: ['Line heights, as unitless multiples of the font size.'],
    doc: 'Line heights.',
    prefix: 'lh-',
    fields: [
      ['tight', 'lh-tight'],
      ['snug', 'lh-snug'],
      ['normal', 'lh-normal'],
      ['relaxed', 'lh-relaxed'],
    ],
  },
  {
    field: 'fontWeight',
    type: 'ThemeFontWeight',
    typeDoc: ['Font weights.'],
    doc: 'Font weights.',
    prefix: 'fw-',
    fields: [
      ['regular', 'fw-regular'],
      ['medium', 'fw-medium'],
      ['semibold', 'fw-semibold'],
      ['bold', 'fw-bold'],
    ],
  },
  {
    field: 'tracking',
    type: 'ThemeTracking',
    typeDoc: ['Letter spacing.'],
    doc: 'Letter spacing.',
    prefix: 'tracking-',
    fields: [
      ['tight', 'tracking-tight'],
      ['normal', 'tracking-normal'],
      ['wide', 'tracking-wide'],
      ['eyebrow', 'tracking-eyebrow'],
    ],
  },
  {
    field: 'space',
    type: 'ThemeSpace',
    typeDoc: ['The spacing ramp, keyed by its step: `space[4]` is four steps of the 4px base.'],
    doc: 'The spacing ramp.',
    prefix: 'sp-',
    fields: [
      ['1', 'sp-1'],
      ['2', 'sp-2'],
      ['3', 'sp-3'],
      ['4', 'sp-4'],
      ['5', 'sp-5'],
      ['6', 'sp-6'],
      ['8', 'sp-8'],
      ['10', 'sp-10'],
      ['12', 'sp-12'],
      ['16', 'sp-16'],
    ],
  },
  {
    field: 'radius',
    type: 'ThemeRadius',
    typeDoc: ['Corner radii.'],
    doc: 'Corner radii.',
    prefix: 'r-',
    fields: [
      ['sm', 'r-sm'],
      ['md', 'r-md'],
      ['lg', 'r-lg'],
      ['pill', 'r-pill'],
    ],
  },
  {
    field: 'shadow',
    type: 'ThemeShadow',
    typeDoc: ['Shadows. Both schemes are flat today, and the tokens carry that.'],
    doc: 'Shadows.',
    prefix: 'shadow-',
    fields: [
      ['sm', 'shadow-sm'],
      ['md', 'shadow-md'],
    ],
  },
  {
    field: 'easing',
    type: 'ThemeEasing',
    typeDoc: ['Easing curves.'],
    doc: 'Easing curves.',
    prefix: 'ease-',
    fields: [['out', 'ease-out']],
  },
  {
    field: 'duration',
    type: 'ThemeDuration',
    typeDoc: ['Motion durations, keyed by step: 1 is the fastest.'],
    doc: 'Motion durations.',
    prefix: 'dur-',
    fields: [
      ['1', 'dur-1'],
      ['2', 'dur-2'],
      ['3', 'dur-3'],
    ],
  },
  {
    field: 'control',
    type: 'ThemeControl',
    typeDoc: ['Control heights — the height a button, input or select stands at.'],
    doc: 'Control heights.',
    prefix: 'h-',
    fields: [
      ['sm', 'h-sm'],
      ['md', 'h-md'],
      ['lg', 'h-lg'],
      ['xl', 'h-xl'],
    ],
  },
  {
    field: 'shell',
    type: 'ThemeShell',
    typeDoc: ['Shell dimensions — the app-shell grid and the mobile drawer share these.'],
    doc: 'Shell dimensions.',
    prefix: '',
    fields: [
      ['sidebarW', 'sidebar-w'],
      ['topbarH', 'topbar-h'],
    ],
  },
];

const SCHEMES = [
  { scheme: 'dark', name: 'forge-dark', doc: 'dark ramp — the `:root` block' },
  { scheme: 'light', name: 'forge-light', doc: 'light ramp — the `[data-theme="light"]` block' },
];

/* ------------------------------------------------------------ TypeScript text */

/** A string as a TypeScript literal, in whichever quote needs no escape. */
export function quote(text) {
  const escaped = String(text).replace(/\\/g, '\\\\');
  if (escaped.includes("'") && !escaped.includes('"')) return `"${escaped}"`;
  return `'${escaped.replace(/'/g, "\\'")}'`;
}

const IDENTIFIER = /^[A-Za-z_$][A-Za-z0-9_$]*$/;
const INDEX = /^\d+$/;

/** A field as it is written where a property name goes. */
const property = (field) => (IDENTIFIER.test(field) || INDEX.test(field) ? field : quote(field));

/** A field as it is written where the property is read back off its group. */
const lookup = (field) =>
  IDENTIFIER.test(field) ? `.${field}` : `[${INDEX.test(field) ? field : quote(field)}]`;

/** A doc comment: one line when it fits on one, a block when it does not. */
function doc(lines, indent) {
  if (lines.length === 1) return [`${indent}/** ${lines[0]} */`];
  return [`${indent}/**`, ...lines.map((line) => `${indent} *${line ? ` ${line}` : ''}`), `${indent} */`];
}

/**
 * The column an emitted literal wraps at. Wider than the 80 columns prose runs
 * to: a font stack is one unbreakable value and reads worse split.
 */
const WIDTH = 100;

/** `label[a, b, c],` on one line, or one entry per line when that overruns. */
function list(label, entries, indent) {
  const oneLine = `${indent}${label}[${entries.join(', ')}],`;
  if (oneLine.length <= WIDTH) return [oneLine];
  return [`${indent}${label}[`, ...entries.map((e) => `${indent}  ${e},`), `${indent}],`];
}

/** `label{ a: 1, b: 2 },` on one line, or one entry per line when that overruns. */
function object(label, entries, indent) {
  const pairs = entries.map(([key, value]) => `${key}: ${value}`);
  const oneLine = `${indent}${label}{ ${pairs.join(', ')} },`;
  if (oneLine.length <= WIDTH) return [oneLine];
  return [`${indent}${label}{`, ...pairs.map((p) => `${indent}  ${p},`), `${indent}},`];
}

/* ---------------------------------------------------------------- the layout */

/** Every token the theme carries, in emission order. */
export const themeTokens = () => [
  ...RAMPS.flatMap((ramp) => ramp.names),
  ...GROUPS.flatMap((group) => group.fields.map(([, name]) => name)),
];

/**
 * Refuse a layout that misses a declared token, or claims one twice.
 *
 * This is what widens the theme and holds it wide. A token added to the source
 * and forgotten here would be reachable only through the `vars` escape hatch —
 * the state this generator exists to end — and `just check` would stay green,
 * because the committed file would still match what this module emits.
 *
 * @param {string[]} [claimed] the tokens the layout names, in emission order.
 */
export function checkCoverage(claimed = themeTokens()) {
  const twice = claimed.filter((name, i) => claimed.indexOf(name) !== i);
  if (twice.length) {
    throw new Error(`${twice.map((n) => `--${n}`).join(', ')} fills more than one field of the typed theme`);
  }
  const missing = tokens.filter((token) => inKit(token, KIT) && !claimed.includes(token.name));
  if (missing.length) {
    throw new Error(
      `the typed theme has no field for ${missing.map((t) => `--${t.name}`).join(', ')}, declared in ` +
        `${SOURCE_PATH}. Add each to GROUPS in ${LAYOUT_PATH}, or scope the token to another kit.`,
    );
  }
}

/** The CSS text a token declares in a scheme — the same string the stylesheet gets. */
const css = (name, scheme) => formatValue(valueFor(tokenNamed(name, KIT), scheme));

/** The doc comment of one field: the prose the layout authored, or the token it carries. */
function fieldDoc([, name, authored]) {
  if (authored) return authored;
  const { note } = tokenNamed(name, KIT);
  return `\`--${name}\`${note ? ` — ${note}.` : ''}`;
}

/* -------------------------------------------------------------------- render */

/** One group's interface. */
function declaration(group) {
  return [
    ...doc(group.typeDoc, ''),
    `export interface ${group.type} {`,
    ...group.fields.flatMap((field) => [
      ...doc([fieldDoc(field)], '  '),
      `  ${property(field[0])}: string;`,
    ]),
    '}',
  ];
}

/**
 * Every interface, emitted once each.
 *
 * The four semantic tones share `SemanticTriple`. A second group claiming a
 * type it does not match would emit one of the two shapes and type-check the
 * other against it, so refuse instead.
 */
function declarations() {
  const emitted = new Map();
  const lines = [];
  for (const group of GROUPS) {
    const shape = JSON.stringify([group.typeDoc, group.fields.map((f) => [f[0], fieldDoc(f)])]);
    const seen = emitted.get(group.type);
    if (seen === undefined) {
      emitted.set(group.type, shape);
      lines.push('', ...declaration(group));
    } else if (seen !== shape) {
      throw new Error(`${group.field} and an earlier group both declare ${group.type}, with different fields`);
    }
  }
  return lines;
}

/** The `Theme` interface: the ramps, every group, then the escape hatch. */
function themeInterface() {
  const lines = [
    ...doc(
      [
        'A complete theme: every token the web kit declares, as a typed value.',
        '',
        'A theme is applied whole. `defineTheme` derives one from another with a',
        'partial override, which is how a brand changes the accent without',
        'restating the rest of the set.',
      ],
      '',
    ),
    'export interface Theme {',
    '  /** Distinguishes one theme from another in a picker. Not read by the engine. */',
    '  name: string;',
    '  /** Base scheme the theme derives from — controls `data-theme` and `color-scheme`. */',
    "  scheme: 'dark' | 'light';",
  ];

  for (const ramp of RAMPS) {
    const first = tokenNamed(ramp.names[0], KIT);
    const last = tokenNamed(ramp.names[ramp.names.length - 1], KIT);
    lines.push(
      ...doc(
        [`${ramp.doc}, \`--${first.name}\` (${first.note}) → \`--${last.name}\` (${last.note}).`],
        '  ',
      ),
      `  ${ramp.field}: [${ramp.names.map(() => 'string').join(', ')}];`,
    );
  }

  for (const group of GROUPS) {
    lines.push(...doc([group.doc], '  '), `  ${group.field}: ${group.type};`);
  }

  lines.push(
    ...doc(
      [
        'Escape hatch: custom properties written verbatim, after the tokens.',
        '',
        'Every declared token has a typed field above, so this is for the',
        'per-instance properties the token source does not declare — a value one',
        'component computes for itself, not a token every kit shares.',
      ],
      '  ',
    ),
    '  vars?: Record<`--${string}`, string>;',
    '}',
  );
  return lines;
}

/** One built-in theme, as the literal that satisfies `Theme`. */
function themeLiteral({ scheme, name, doc: prose }) {
  const lines = [
    ...doc([`The built-in ${prose} of \`css/tokens.css\`, as a value.`], ''),
    `export const ${scheme}Theme: Theme = {`,
    `  name: ${quote(name)},`,
    `  scheme: ${quote(scheme)},`,
  ];
  for (const ramp of RAMPS) {
    lines.push(...list(`${ramp.field}: `, ramp.names.map((n) => quote(css(n, scheme))), '  '));
  }
  for (const group of GROUPS) {
    const entries = group.fields.map(([field, token]) => [property(field), quote(css(token, scheme))]);
    lines.push(...object(`${group.field}: `, entries, '  '));
  }
  lines.push('};');
  return lines;
}

/** `themeToVars`: one entry per declared token, then the escape hatch. */
function toVars() {
  const entries = [
    ...RAMPS.flatMap((ramp) =>
      ramp.names.map((name, i) => `    ${quote(`--${name}`)}: t.${ramp.field}[${i}],`),
    ),
    ...GROUPS.flatMap((group) =>
      group.fields.map(([field, name]) => `    ${quote(`--${name}`)}: t.${group.field}${lookup(field)},`),
    ),
  ];
  return [
    ...doc(
      [
        'Flatten a theme into the custom properties the stylesheets read.',
        '',
        'Every declared token is emitted, so the result overrides the whole of',
        'the `[data-theme]` block it is written over rather than a subset of it.',
        'Anything in `vars` is written last and thus wins.',
      ],
      '',
    ),
    'export function themeToVars(t: Theme): Record<string, string> {',
    '  const vars: Record<string, string> = {',
    ...entries,
    '  };',
    '  if (t.vars) Object.assign(vars, t.vars);',
    '  return vars;',
    '}',
  ];
}

/** @returns {string} the full text of `packages/tokens/src/theme.gen.ts`. */
export function renderThemeTs() {
  checkCoverage();

  const lines = [
    `/* ${bannerLines().join('\n   ')} */`,
    '',
    ...doc(
      [
        'The Forge tokens as a typed value.',
        '',
        'Every token the web kit declares is a field of `Theme`, and `themeToVars`',
        'turns a theme back into the custom properties the stylesheets read. The',
        'engine that applies one — `defineTheme` and `applyTheme` — is behaviour',
        'rather than values, so it lives in `./theme.ts`, which re-exports this',
        'module.',
      ],
      '',
    ),
    ...declarations(),
    '',
    ...themeInterface(),
  ];

  for (const scheme of SCHEMES) lines.push('', ...themeLiteral(scheme));
  lines.push('', ...toVars());

  return `${lines.join('\n')}\n`;
}
