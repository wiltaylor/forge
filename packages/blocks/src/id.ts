/* Block ids. Its own module because `types.gen.ts` mints them and nothing
   else in `types.ts` may be imported from generated code. */

let counter = 0;

/** A fresh block id (web side uses UUIDs; any unique string is valid). */
export function newId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `blk_${Date.now().toString(36)}_${(counter++).toString(36)}`;
}
