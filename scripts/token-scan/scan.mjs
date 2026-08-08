/**
 * Find every reference to a Forge design token, and judge it.
 *
 * The defect this exists for: a `var()` reference to a custom property nobody
 * declared, and with no fallback, makes the whole declaration invalid. The
 * property then takes its inherited value. A dozen borders in the block editor
 * resolved to `currentColor` that way, and nothing said so.
 *
 * Two rules:
 *
 *   1. A referenced name is a declared token, or one of the per-instance
 *      properties in `PER_INSTANCE`. Anything else fails.
 *   2. A reference to a declared token carries no fallback. The fallback is a
 *      second copy of the token's value, free to drift — and every stylesheet
 *      here ships beside `tokens.css`, so none of them needs one. The files in
 *      `FALLBACK_EXEMPT` keep theirs, for the reason each one states.
 *
 * References are written two ways, so both are read: `var(--name)` in a
 * stylesheet or in a style string, and `'--name'` where JavaScript reads a
 * property through a probe element or writes one through an inline style.
 */

/** A `var()` reference. The trailing comma, if any, opens the fallback. */
const VAR_REF = /var\(\s*(--[A-Za-z0-9_-]+)\s*(,)?/g;

/**
 * A quoted property name, and — without consuming it, so that the next name is
 * still matched — the argument after it. Another property name there is the
 * second half of a `color-mix()`, not a fallback.
 */
const QUOTED_REF = /(['"])(--[a-z][A-Za-z0-9_-]*)\1(?=(?:\s*,\s*(['"])([^'"]*)\3)?)/g;

/**
 * Custom properties that carry a value per instance rather than a token value.
 * Each is written by the source named here, and read with a fallback that is
 * its default, so the rules above would otherwise call it undeclared.
 */
export const PER_INSTANCE = [
  {
    name: '--fbk-indent',
    set: 'packages/blocks/src/render.tsx',
    why: 'the nesting depth of one list item',
  },
  {
    name: '--fbk-cols',
    set: 'packages/blocks/src/tableedit.tsx',
    why: 'the column count of one table',
  },
  {
    name: '--fslider-fill',
    set: 'packages/ui/src/forms.tsx',
    why: 'the filled fraction of one slider',
  },
  {
    name: '--fgraph-node-w',
    set: 'the consuming app',
    why: 'the node width of one graph, documented as an override in the graph kit',
  },
];

/**
 * The files whose token references keep a hand-written fallback on purpose.
 *
 * Deliberate divergences are declared here rather than discovered by the next
 * reader, who would otherwise "fix" one into drift.
 */
export const FALLBACK_EXEMPT = [
  {
    file: 'packages/term/src/theme.ts',
    why:
      'xterm paints to a canvas, so the terminal kit resolves each token through a probe ' +
      'element and needs a concrete colour for when that resolution fails. The accent ' +
      'stand-in, #5A8FDB, is hand-picked and deliberately not the token value.',
  },
];

/**
 * Blank out comments, so that a token name written in prose is not a reference.
 *
 * Every newline survives, so line numbers still hold. A quote opens a string,
 * where `/*` and `//` are text rather than a comment — `content: "// "` and a
 * URL both rely on that.
 *
 * @param {string} text
 * @param {{ lineComments?: boolean }} options `//` runs to the end of the line.
 * @returns {string}
 */
export function stripComments(text, { lineComments = false } = {}) {
  const out = [...text];
  const blank = (from, to) => {
    for (let i = from; i < to; i++) if (out[i] !== '\n') out[i] = ' ';
  };

  for (let i = 0; i < text.length; i++) {
    const two = text.slice(i, i + 2);
    if (two === '/*') {
      const end = text.indexOf('*/', i + 2);
      const stop = end === -1 ? text.length : end + 2;
      blank(i, stop);
      i = stop - 1;
      continue;
    }
    if (lineComments && two === '//') {
      let end = text.indexOf('\n', i);
      if (end === -1) end = text.length;
      blank(i, end);
      i = end - 1;
      continue;
    }
    if (text[i] === "'" || text[i] === '"' || text[i] === '`') {
      const quote = text[i];
      for (i++; i < text.length && text[i] !== quote; i++) if (text[i] === '\\') i++;
    }
  }
  return out.join('');
}

/** The 1-based line each offset falls on. */
function lineNumbers(text) {
  const starts = [0];
  for (let i = text.indexOf('\n'); i !== -1; i = text.indexOf('\n', i + 1)) starts.push(i + 1);
  return (index) => {
    let lo = 0;
    let hi = starts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (starts[mid] <= index) lo = mid;
      else hi = mid - 1;
    }
    return lo + 1;
  };
}

/**
 * Every token reference in one file, in source order.
 *
 * @param {string} text
 * @param {{ lineComments?: boolean }} options
 * @returns {{ name: string, fallback: boolean, line: number }[]}
 */
export function references(text, { lineComments = false } = {}) {
  const source = stripComments(text, { lineComments });
  const lineOf = lineNumbers(source);
  const found = [];
  for (const m of source.matchAll(VAR_REF)) {
    found.push({ name: m[1], fallback: m[2] === ',', index: m.index });
  }
  for (const m of source.matchAll(QUOTED_REF)) {
    const next = m[4];
    found.push({ name: m[2], fallback: next !== undefined && !next.startsWith('--'), index: m.index });
  }
  found.sort((a, b) => a.index - b.index);
  return found.map(({ name, fallback, index }) => ({ name, fallback, line: lineOf(index) }));
}

/** The extension of a path, including the dot. */
export const extensionOf = (path) => {
  const dot = path.lastIndexOf('.');
  return dot === -1 ? '' : path.slice(dot);
};

/**
 * The file types that hold token references, and whether `//` opens a comment
 * in each. Everything else in the tree holds none.
 */
export const SCANNED = { '.css': false, '.ts': true, '.tsx': true };

/** True when a file of this path is worth scanning. */
export const isScanned = (path) => extensionOf(path) in SCANNED;

/**
 * Judge every reference in every file.
 *
 * A dead entry in either table is a failure of its own, so that neither list
 * outlives the reference it excuses.
 *
 * @param {{ path: string, text: string }[]} files
 * @param {Set<string>} declared every token name the source declares for the web kit
 * @returns {{ path: string, line: number, name: string, problem: string }[]}
 */
export function violations(files, declared) {
  const perInstance = new Map(PER_INSTANCE.map((entry) => [entry.name, entry]));
  const exempt = new Map(FALLBACK_EXEMPT.map((entry) => [entry.file, entry]));
  const usedNames = new Set();
  const usedExemptions = new Set();
  const found = [];

  for (const file of files) {
    const lineComments = SCANNED[extensionOf(file.path)];
    for (const ref of references(file.text, { lineComments })) {
      const where = { path: file.path, line: ref.line, name: ref.name };
      if (!declared.has(ref.name)) {
        if (!perInstance.has(ref.name)) {
          found.push({ ...where, problem: 'no token of this name is declared' });
          continue;
        }
        usedNames.add(ref.name);
        continue;
      }
      if (!ref.fallback) continue;
      if (exempt.has(file.path)) {
        usedExemptions.add(file.path);
        continue;
      }
      found.push({ ...where, problem: 'a declared token needs no fallback' });
    }
  }

  for (const entry of PER_INSTANCE) {
    if (usedNames.has(entry.name)) continue;
    found.push({
      path: 'scripts/token-scan/scan.mjs',
      line: 0,
      name: entry.name,
      problem: 'nothing references this per-instance property; drop the allowlist entry',
    });
  }
  for (const entry of FALLBACK_EXEMPT) {
    if (usedExemptions.has(entry.file)) continue;
    found.push({
      path: 'scripts/token-scan/scan.mjs',
      line: 0,
      name: entry.file,
      problem: 'this file carries no fallback any more; drop the exemption',
    });
  }
  return found;
}
