/**
 * The pieces every generated Rust file is built from.
 *
 * Both kits' palettes and the egui token set share a module header, a comment
 * column, and one spelling of an `f32` literal, so they live here rather than
 * in whichever generator happened to need them first.
 */
import { bannerLines } from './banner.mjs';

/** The `//!` module header: the generated-file banner, then the module's own prose. */
export const moduleDoc = (prose) =>
  [...bannerLines(), '', ...prose].map((line) => (line ? `//! ${line}` : '//!'));

/** Lay out `code // comment` rows, comments aligned one space past the widest row. */
export function aligned(rows, indent) {
  const width = Math.max(0, ...rows.filter((row) => row.comment).map((row) => row.code.length));
  return rows.map((row) =>
    row.comment ? `${indent}${row.code.padEnd(width)} // ${row.comment}` : `${indent}${row.code}`,
  );
}

/**
 * A number as an `f32` literal.
 *
 * Rust reads `4` as an integer, so a whole number takes a `.0`. Everything else
 * prints as JavaScript writes it, which is the shortest text that round-trips.
 */
export const f32 = (n) => (Number.isInteger(n) ? `${n}.0` : String(n));
