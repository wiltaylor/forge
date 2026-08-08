/**
 * The pieces every generated TypeScript file is built from.
 *
 * The block kind generators and the token theme generator share one spelling of
 * a string literal, one rule for when a key needs quoting, one column to wrap
 * at, and one JSDoc style, so those live here rather than in whichever
 * generator happened to need them first. `./rust.mjs` is the same thing for the
 * Rust kits.
 */

/**
 * A TypeScript string literal, in whichever quote it needs no escape in.
 *
 * Single, like the rest of the kit, unless the text holds an apostrophe and no
 * double quote — a font stack reads worse with every quote inside it escaped.
 */
export function quote(text) {
  const escaped = text.replace(/\\/g, '\\\\');
  if (escaped.includes("'") && !escaped.includes('"')) return `"${escaped}"`;
  return `'${escaped.replace(/'/g, "\\'")}'`;
}

/**
 * Whether `key` stands as an object key unquoted — an identifier, or the digits
 * of a ramp step. `{ 4: '16px' }` is read back as `space[4]`, which is the step
 * the token source names.
 */
export const plainKey = (key) => /^[A-Za-z_$][\w$]*$/.test(key) || /^\d+$/.test(key);

/** A key as it is written where a property name goes. */
export const propertyKey = (key) => (plainKey(key) ? key : quote(key));

/** The width the emitted TypeScript wraps at. */
export const PRINT_WIDTH = 100;

/** Wrap `lines` in a block comment at `indent`, in the kit's JSDoc style. */
export function docComment(lines, indent = '') {
  if (!lines.length) return [];
  if (lines.length === 1) return [`${indent}/** ${lines[0]} */`];
  const body = lines.map((line, i) => {
    if (i === 0) return `${indent}/** ${line}`;
    // A paragraph break carries no indent of its own: trailing space is noise.
    return line ? `${indent}    ${line}` : '';
  });
  body[body.length - 1] += ' */';
  return body;
}
