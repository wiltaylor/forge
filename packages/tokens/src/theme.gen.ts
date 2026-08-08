/* GENERATED FILE — do not edit by hand.
   Source:     packages/tokens/tokens.source.mjs
   Regenerate: just generate   (`just check` fails while this file is stale) */

/** The Forge tokens as a typed value.

    Every token the web kit declares is a field of `Theme`, and `themeToVars`
    turns a theme back into the custom properties the stylesheets read. The
    engine that applies one — `defineTheme` and `applyTheme` — is behaviour
    rather than values, so it lives in `./theme.ts`, which re-exports this
    module. */

/** Border strengths, subtle → strong. */
export interface ThemeBorder {
  /** `--border-subtle` */
  subtle: string;
  /** `--border` */
  default: string;
  /** `--border-strong` */
  strong: string;
}

/** The accent colour and the states it takes. */
export interface ThemeAccent {
  /** `--accent` */
  base: string;
  /** `--accent-hover` */
  hover: string;
  /** `--accent-press` */
  press: string;
  /** `--accent-bg` */
  bg: string;
  /** `--accent-fg` */
  fg: string;
  /** `--accent-contrast` — text on solid accent. */
  contrast: string;
}

/** One semantic tone: the colour itself, the tint it sits on, and the text on that tint. */
export interface SemanticTriple {
  /** Solid tone colour (borders, icons, strokes). */
  base: string;
  /** Translucent tint used as a background. */
  bg: string;
  /** Text colour readable on the tint. */
  fg: string;
}

/** The two font stacks. */
export interface ThemeFonts {
  /** `--font-sans` */
  sans: string;
  /** `--font-mono` */
  mono: string;
}

/** The type scale. `xs`..`lg` are the body sizes; `xl`..`3xl` are the headings. */
export interface ThemeFontSize {
  /** `--fs-xs` */
  xs: string;
  /** `--fs-sm` */
  sm: string;
  /** `--fs-base` */
  base: string;
  /** `--fs-md` */
  md: string;
  /** `--fs-lg` */
  lg: string;
  /** `--fs-xl` */
  xl: string;
  /** `--fs-2xl` */
  '2xl': string;
  /** `--fs-3xl` */
  '3xl': string;
}

/** Line heights, as unitless multiples of the font size. */
export interface ThemeLineHeight {
  /** `--lh-tight` */
  tight: string;
  /** `--lh-snug` */
  snug: string;
  /** `--lh-normal` */
  normal: string;
  /** `--lh-relaxed` */
  relaxed: string;
}

/** Font weights. */
export interface ThemeFontWeight {
  /** `--fw-regular` */
  regular: string;
  /** `--fw-medium` */
  medium: string;
  /** `--fw-semibold` */
  semibold: string;
  /** `--fw-bold` */
  bold: string;
}

/** Letter spacing. */
export interface ThemeTracking {
  /** `--tracking-tight` */
  tight: string;
  /** `--tracking-normal` */
  normal: string;
  /** `--tracking-wide` */
  wide: string;
  /** `--tracking-eyebrow` */
  eyebrow: string;
}

/** The spacing ramp, keyed by its step: `space[4]` is four steps of the 4px base. */
export interface ThemeSpace {
  /** `--sp-1` */
  1: string;
  /** `--sp-2` */
  2: string;
  /** `--sp-3` */
  3: string;
  /** `--sp-4` */
  4: string;
  /** `--sp-5` */
  5: string;
  /** `--sp-6` */
  6: string;
  /** `--sp-8` */
  8: string;
  /** `--sp-10` */
  10: string;
  /** `--sp-12` */
  12: string;
  /** `--sp-16` */
  16: string;
}

/** Corner radii. */
export interface ThemeRadius {
  /** `--r-sm` */
  sm: string;
  /** `--r-md` */
  md: string;
  /** `--r-lg` */
  lg: string;
  /** `--r-pill` */
  pill: string;
}

/** Shadows. Both schemes are flat today, and the tokens carry that. */
export interface ThemeShadow {
  /** `--shadow-sm` */
  sm: string;
  /** `--shadow-md` */
  md: string;
}

/** Easing curves. */
export interface ThemeEasing {
  /** `--ease-out` */
  out: string;
}

/** Motion durations, keyed by step: 1 is the fastest. */
export interface ThemeDuration {
  /** `--dur-1` */
  1: string;
  /** `--dur-2` */
  2: string;
  /** `--dur-3` */
  3: string;
}

/** Control heights — the height a button, input or select stands at. */
export interface ThemeControl {
  /** `--h-sm` */
  sm: string;
  /** `--h-md` */
  md: string;
  /** `--h-lg` */
  lg: string;
  /** `--h-xl` */
  xl: string;
}

/** Shell dimensions — the app-shell grid and the mobile drawer share these. */
export interface ThemeShell {
  /** `--sidebar-w` */
  sidebarW: string;
  /** `--topbar-h` */
  topbarH: string;
}

/** The z-index scale, bottom to top. The modal and the command palette share
    `modal`: both are modal surfaces, so only one of them is open at a time. */
export interface ThemeLayer {
  /** `--layer-topbar` — sticky shell chrome. */
  topbar: string;
  /** `--layer-scrim` — backdrop behind the drawer. */
  scrim: string;
  /** `--layer-drawer` — off-canvas sidebar. */
  drawer: string;
  /** `--layer-sheet` — above the drawer, below the modal. */
  sheet: string;
  /** `--layer-modal` — modal and command palette. */
  modal: string;
  /** `--layer-pop` — anchored popovers; above the modal so they work inside one. */
  pop: string;
  /** `--layer-toast` — above the modal. */
  toast: string;
  /** `--layer-tip` — tooltips top every interactive surface. */
  tip: string;
  /** `--layer-fx` — non-interactive particle canvas. */
  fx: string;
}

/** A complete theme: every token the web kit declares, as a typed value.

    A theme is applied whole. `defineTheme` derives one from another with a
    partial override, which is how a brand changes the accent without
    restating the rest of the set. */
export interface Theme {
  /** Distinguishes one theme from another in a picker. Not read by the engine. */
  name: string;
  /** Base scheme the theme derives from — controls `data-theme` and `color-scheme`. */
  scheme: 'dark' | 'light';
  /** Backgrounds, `--bg-0` (page) → `--bg-4` (popover, dropdown). */
  bg: [string, string, string, string, string];
  /** Foregrounds, `--fg-0` (primary text) → `--fg-3` (disabled, placeholder). */
  fg: [string, string, string, string];
  /** Borders. */
  border: ThemeBorder;
  /** The accent colour. */
  accent: ThemeAccent;
  /** The success tone. */
  success: SemanticTriple;
  /** The warning tone. */
  warning: SemanticTriple;
  /** The danger tone. */
  danger: SemanticTriple;
  /** The info tone. */
  info: SemanticTriple;
  /** Font stacks. */
  fonts: ThemeFonts;
  /** The type scale. */
  fontSize: ThemeFontSize;
  /** Line heights. */
  lineHeight: ThemeLineHeight;
  /** Font weights. */
  fontWeight: ThemeFontWeight;
  /** Letter spacing. */
  tracking: ThemeTracking;
  /** The spacing ramp. */
  space: ThemeSpace;
  /** Corner radii. */
  radius: ThemeRadius;
  /** Shadows. */
  shadow: ThemeShadow;
  /** Easing curves. */
  easing: ThemeEasing;
  /** Motion durations. */
  duration: ThemeDuration;
  /** Control heights. */
  control: ThemeControl;
  /** Shell dimensions. */
  shell: ThemeShell;
  /** The z-index scale. */
  layer: ThemeLayer;
  /** Escape hatch: custom properties written verbatim, after the tokens.

      Every declared token has a typed field above, so this is for the
      per-instance properties the token source does not declare — a value one
      component computes for itself, not a token every kit shares. */
  vars?: Record<`--${string}`, string>;
}

/** The built-in dark ramp — the `:root` block of `css/tokens.css`, as a value. */
export const darkTheme: Theme = {
  name: 'forge-dark',
  scheme: 'dark',
  bg: ['#0B0D10', '#11141A', '#171B22', '#1E232C', '#252B36'],
  fg: ['#ECEEF2', '#B7BDC8', '#7C8593', '#4E5664'],
  border: { subtle: '#1A1F27', default: '#262C36', strong: '#3A4250' },
  accent: {
    base: 'oklch(0.62 0.16 250)',
    hover: 'oklch(0.66 0.17 250)',
    press: 'oklch(0.56 0.16 250)',
    bg: 'oklch(0.62 0.16 250 / 0.14)',
    fg: 'oklch(0.82 0.13 250)',
    contrast: '#FFFFFF',
  },
  success: {
    base: 'oklch(0.68 0.14 150)',
    bg: 'oklch(0.68 0.14 150 / 0.14)',
    fg: 'oklch(0.82 0.16 150)',
  },
  warning: {
    base: 'oklch(0.78 0.14 75)',
    bg: 'oklch(0.78 0.14 75 / 0.14)',
    fg: 'oklch(0.86 0.13 80)',
  },
  danger: {
    base: 'oklch(0.65 0.20 25)',
    bg: 'oklch(0.65 0.20 25 / 0.14)',
    fg: 'oklch(0.78 0.16 25)',
  },
  info: {
    base: 'oklch(0.68 0.13 230)',
    bg: 'oklch(0.68 0.13 230 / 0.14)',
    fg: 'oklch(0.82 0.12 230)',
  },
  fonts: {
    sans: "'IBM Plex Sans', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
    mono: "'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, Consolas, monospace",
  },
  fontSize: {
    xs: '11px',
    sm: '12px',
    base: '14px',
    md: '16px',
    lg: '18px',
    xl: '22px',
    '2xl': '28px',
    '3xl': '34px',
  },
  lineHeight: { tight: '1.2', snug: '1.4', normal: '1.5', relaxed: '1.65' },
  fontWeight: { regular: '400', medium: '500', semibold: '600', bold: '700' },
  tracking: { tight: '-0.01em', normal: '0', wide: '0.04em', eyebrow: '0.08em' },
  space: {
    1: '4px',
    2: '8px',
    3: '12px',
    4: '16px',
    5: '20px',
    6: '24px',
    8: '32px',
    10: '40px',
    12: '48px',
    16: '64px',
  },
  radius: { sm: '4px', md: '6px', lg: '8px', pill: '999px' },
  shadow: { sm: 'none', md: 'none' },
  easing: { out: 'cubic-bezier(0.2, 0, 0, 1)' },
  duration: { 1: '80ms', 2: '160ms', 3: '240ms' },
  control: { sm: '28px', md: '32px', lg: '36px', xl: '40px' },
  shell: { sidebarW: '240px', topbarH: '48px' },
  layer: {
    topbar: '10',
    scrim: '20',
    drawer: '30',
    sheet: '40',
    modal: '50',
    pop: '60',
    toast: '70',
    tip: '80',
    fx: '90',
  },
};

/** The built-in light ramp — the `[data-theme="light"]` block of `css/tokens.css`, as a value. */
export const lightTheme: Theme = {
  name: 'forge-light',
  scheme: 'light',
  bg: ['#FAFAFA', '#FFFFFF', '#F4F5F7', '#EAECEF', '#FFFFFF'],
  fg: ['#0C0F14', '#3D4654', '#6B7383', '#A0A6B2'],
  border: { subtle: '#EEF0F3', default: '#DCDFE4', strong: '#B6BBC4' },
  accent: {
    base: 'oklch(0.52 0.18 250)',
    hover: 'oklch(0.46 0.19 250)',
    press: 'oklch(0.40 0.19 250)',
    bg: 'oklch(0.55 0.17 250 / 0.14)',
    fg: 'oklch(0.38 0.19 250)',
    contrast: '#FFFFFF',
  },
  success: {
    base: 'oklch(0.50 0.15 150)',
    bg: 'oklch(0.55 0.15 150 / 0.16)',
    fg: 'oklch(0.36 0.14 150)',
  },
  warning: {
    base: 'oklch(0.62 0.16 70)',
    bg: 'oklch(0.65 0.16 70 / 0.20)',
    fg: 'oklch(0.40 0.14 60)',
  },
  danger: {
    base: 'oklch(0.52 0.22 25)',
    bg: 'oklch(0.55 0.21 25 / 0.14)',
    fg: 'oklch(0.42 0.20 25)',
  },
  info: {
    base: 'oklch(0.50 0.14 230)',
    bg: 'oklch(0.55 0.14 230 / 0.16)',
    fg: 'oklch(0.36 0.13 230)',
  },
  fonts: {
    sans: "'IBM Plex Sans', ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
    mono: "'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, Consolas, monospace",
  },
  fontSize: {
    xs: '11px',
    sm: '12px',
    base: '14px',
    md: '16px',
    lg: '18px',
    xl: '22px',
    '2xl': '28px',
    '3xl': '34px',
  },
  lineHeight: { tight: '1.2', snug: '1.4', normal: '1.5', relaxed: '1.65' },
  fontWeight: { regular: '400', medium: '500', semibold: '600', bold: '700' },
  tracking: { tight: '-0.01em', normal: '0', wide: '0.04em', eyebrow: '0.08em' },
  space: {
    1: '4px',
    2: '8px',
    3: '12px',
    4: '16px',
    5: '20px',
    6: '24px',
    8: '32px',
    10: '40px',
    12: '48px',
    16: '64px',
  },
  radius: { sm: '4px', md: '6px', lg: '8px', pill: '999px' },
  shadow: { sm: 'none', md: 'none' },
  easing: { out: 'cubic-bezier(0.2, 0, 0, 1)' },
  duration: { 1: '80ms', 2: '160ms', 3: '240ms' },
  control: { sm: '28px', md: '32px', lg: '36px', xl: '40px' },
  shell: { sidebarW: '240px', topbarH: '48px' },
  layer: {
    topbar: '10',
    scrim: '20',
    drawer: '30',
    sheet: '40',
    modal: '50',
    pop: '60',
    toast: '70',
    tip: '80',
    fx: '90',
  },
};

/** Flatten a theme into the custom properties the stylesheets read.

    Every declared token is emitted, so the result overrides the whole of
    the `[data-theme]` block it is written over rather than a subset of it.
    Anything in `vars` is written last and thus wins. */
export function themeToVars(t: Theme): Record<string, string> {
  const vars: Record<string, string> = {
    '--bg-0': t.bg[0],
    '--bg-1': t.bg[1],
    '--bg-2': t.bg[2],
    '--bg-3': t.bg[3],
    '--bg-4': t.bg[4],
    '--fg-0': t.fg[0],
    '--fg-1': t.fg[1],
    '--fg-2': t.fg[2],
    '--fg-3': t.fg[3],
    '--border-subtle': t.border.subtle,
    '--border': t.border.default,
    '--border-strong': t.border.strong,
    '--accent': t.accent.base,
    '--accent-hover': t.accent.hover,
    '--accent-press': t.accent.press,
    '--accent-bg': t.accent.bg,
    '--accent-fg': t.accent.fg,
    '--accent-contrast': t.accent.contrast,
    '--success': t.success.base,
    '--success-bg': t.success.bg,
    '--success-fg': t.success.fg,
    '--warning': t.warning.base,
    '--warning-bg': t.warning.bg,
    '--warning-fg': t.warning.fg,
    '--danger': t.danger.base,
    '--danger-bg': t.danger.bg,
    '--danger-fg': t.danger.fg,
    '--info': t.info.base,
    '--info-bg': t.info.bg,
    '--info-fg': t.info.fg,
    '--font-sans': t.fonts.sans,
    '--font-mono': t.fonts.mono,
    '--fs-xs': t.fontSize.xs,
    '--fs-sm': t.fontSize.sm,
    '--fs-base': t.fontSize.base,
    '--fs-md': t.fontSize.md,
    '--fs-lg': t.fontSize.lg,
    '--fs-xl': t.fontSize.xl,
    '--fs-2xl': t.fontSize['2xl'],
    '--fs-3xl': t.fontSize['3xl'],
    '--lh-tight': t.lineHeight.tight,
    '--lh-snug': t.lineHeight.snug,
    '--lh-normal': t.lineHeight.normal,
    '--lh-relaxed': t.lineHeight.relaxed,
    '--fw-regular': t.fontWeight.regular,
    '--fw-medium': t.fontWeight.medium,
    '--fw-semibold': t.fontWeight.semibold,
    '--fw-bold': t.fontWeight.bold,
    '--tracking-tight': t.tracking.tight,
    '--tracking-normal': t.tracking.normal,
    '--tracking-wide': t.tracking.wide,
    '--tracking-eyebrow': t.tracking.eyebrow,
    '--sp-1': t.space[1],
    '--sp-2': t.space[2],
    '--sp-3': t.space[3],
    '--sp-4': t.space[4],
    '--sp-5': t.space[5],
    '--sp-6': t.space[6],
    '--sp-8': t.space[8],
    '--sp-10': t.space[10],
    '--sp-12': t.space[12],
    '--sp-16': t.space[16],
    '--r-sm': t.radius.sm,
    '--r-md': t.radius.md,
    '--r-lg': t.radius.lg,
    '--r-pill': t.radius.pill,
    '--shadow-sm': t.shadow.sm,
    '--shadow-md': t.shadow.md,
    '--ease-out': t.easing.out,
    '--dur-1': t.duration[1],
    '--dur-2': t.duration[2],
    '--dur-3': t.duration[3],
    '--h-sm': t.control.sm,
    '--h-md': t.control.md,
    '--h-lg': t.control.lg,
    '--h-xl': t.control.xl,
    '--sidebar-w': t.shell.sidebarW,
    '--topbar-h': t.shell.topbarH,
    '--layer-topbar': t.layer.topbar,
    '--layer-scrim': t.layer.scrim,
    '--layer-drawer': t.layer.drawer,
    '--layer-sheet': t.layer.sheet,
    '--layer-modal': t.layer.modal,
    '--layer-pop': t.layer.pop,
    '--layer-toast': t.layer.toast,
    '--layer-tip': t.layer.tip,
    '--layer-fx': t.layer.fx,
  };
  if (t.vars) Object.assign(vars, t.vars);
  return vars;
}
