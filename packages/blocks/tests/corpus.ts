/* The block key corpus, read from TypeScript.

   `contract/blocks/corpus.json` is authored data: a starting document, an
   address, a key sequence, and the document that must result. The Rust kits
   read it through `crates/forge-block-corpus`; this is the same reading for
   the web kit, and `block_corpus.test.tsx` is the driver that presses the keys.

   Nothing here knows about an editor. The full set of rules a case must keep
   lives with the authored file, in `Corpus::validate` (`cargo test -p
   forge-block-corpus`); this module checks only the one rule the web runner
   itself depends on — that every case states where the web kit stands. */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Block, BlockDocument } from '../src/types';
import { DOC_VERSION, newId } from '../src/types';

/** Kit id of the web driver. */
export const WEB = 'web';

/* `import.meta.url` is not a file URL here: the JSX transform this suite runs
   under rewrites it for a browser. `import.meta.dirname` survives it. */
const CORPUS_PATH = resolve(import.meta.dirname, '../../../contract/blocks/corpus.json');

/** A known gap between a kit and the case it fails. */
export interface Divergence {
  /** The issue that closes it. */
  issue: number;
  /** What the kit does instead. */
  why: string;
}

/** Where the editor is when the first key arrives. `block` indexes the
    document root; `column` + `index` address a block inside a column cell.
    Then the mode: `caret` for a text caret, `row` + `col` for a table cell
    (display row 0 is the header), neither for block selection. */
export interface At {
  block: number;
  column?: number;
  index?: number;
  caret?: number;
  row?: number;
  col?: number;
}

/** One keypress in the browser `KeyboardEvent` vocabulary: a layout-independent
    `code`, plus the produced character in `key` when the key is printable. */
export interface Key {
  code: string;
  key?: string;
  shift?: boolean;
  ctrl?: boolean;
  alt?: boolean;
}

/** One editing case: a starting document, an address, a key sequence, and the
    document that must result. `doc` and `expect` are blocks without ids. */
export interface Case {
  id: string;
  title: string;
  note?: string;
  applies: string[];
  inapplicable?: Record<string, string>;
  diverges?: Record<string, Divergence>;
  doc: unknown[];
  at: At;
  keys: Key[];
  expect: unknown[];
}

export interface Corpus {
  corpus_version: string;
  kits: string[];
  cases: Case[];
}

/** Parse the authored corpus and check that every case says where the web kit
    stands — in exactly one of `applies`, `inapplicable` or `diverges`. A gap
    has to be written down; it cannot be created by forgetting. */
export function loadCorpus(): Corpus {
  const corpus = JSON.parse(readFileSync(CORPUS_PATH, 'utf8')) as Corpus;
  if (!corpus.kits.includes(WEB)) throw new Error(`the corpus names no kit ${WEB}`);
  for (const c of corpus.cases) {
    const stated = [
      c.applies.includes(WEB),
      WEB in (c.inapplicable ?? {}),
      WEB in (c.diverges ?? {}),
    ].filter(Boolean).length;
    if (stated !== 1)
      throw new Error(
        `${c.id}: kit ${WEB} is stated ${stated} times, not once — see contract/blocks/README.md`,
      );
  }
  return corpus;
}

/** What the web kit must do with a case: produce [`caseExpected`](#caseExpected),
    produce something else (a recorded divergence), or not run it at all. */
export type Verdict = 'match' | 'differ' | 'skip';

export function webVerdict(c: Case): Verdict {
  if (c.applies.includes(WEB)) return 'match';
  if (WEB in (c.diverges ?? {})) return 'differ';
  return 'skip';
}

/** The starting document. Ids are minted here — the corpus does not author
    them, because block identity is not part of the editing policy. */
export function caseDocument(c: Case): BlockDocument {
  return { version: DOC_VERSION, blocks: mapBlocks(c.doc, mintId) as unknown as Block[] };
}

/** The document the keys must produce. */
export function caseExpected(c: Case): BlockDocument {
  return { version: DOC_VERSION, blocks: mapBlocks(c.expect, mintId) as unknown as Block[] };
}

/** A document as the corpus judges it: every block id removed. Block identity
    is editor bookkeeping, not editing policy — two documents that differ only
    by id are the same document to a case. */
export function judged(doc: BlockDocument): unknown {
  return { version: doc.version, blocks: mapBlocks(doc.blocks, dropId) };
}

/** A block as JSON. An authored block arrives without an id, and a judged one
    leaves without one, so the walk below reads both as plain objects. */
type RawBlock = Record<string, unknown>;

const mintId = (block: RawBlock): RawBlock => ({ id: newId(), ...block });
const dropId = ({ id: _id, ...rest }: RawBlock): RawBlock => rest;

/** Apply `f` to every block in a list, down through the cells of any `columns`
    block. One walker, because minting ids and dropping them are the same
    traversal — the shape `walk_blocks` has in `crates/forge-block-corpus`.

    The corpus is data, so this is the one place a block's shape is taken on
    trust; a wrong shape fails the case it is in. */
function mapBlocks(blocks: unknown[], f: (block: RawBlock) => RawBlock): RawBlock[] {
  return blocks.map((b) => {
    const block = f({ ...(b as RawBlock) });
    const columns = block.columns;
    if (Array.isArray(columns))
      block.columns = columns.map((col: RawBlock) => ({
        ...col,
        blocks: mapBlocks((col.blocks as unknown[]) ?? [], f),
      }));
    return block;
  });
}

/** How a key reads in a failure report: `Shift+Tab`, `KeyA "a"`. */
export function keyLabel(key: Key): string {
  const mods = [
    [key.ctrl, 'Ctrl+'],
    [key.alt, 'Alt+'],
    [key.shift, 'Shift+'],
  ] as const;
  const prefix = mods.filter(([on]) => on).map(([, name]) => name).join('');
  return `${prefix}${key.code}${key.key === undefined ? '' : ` ${JSON.stringify(key.key)}`}`;
}

/** A `caret` in the corpus is a **byte** offset into the block's markdown
    source, the unit the Rust editors take. A DOM text field indexes UTF-16
    code units, so a caret past a multi-byte character needs converting. */
export function caretIndex(text: string, byteOffset: number): number {
  const bytes = new TextEncoder().encode(text);
  if (byteOffset >= bytes.length) return text.length;
  return new TextDecoder().decode(bytes.slice(0, byteOffset)).length;
}
