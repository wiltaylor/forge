/* Loading untrusted JSON as a document.

   The Rust kits get this from serde plus `Document::normalize`; the web kit
   gets it from `loadDocument`. These tests pin the three promises: an
   unrecognised version is refused, a malformed block is refused with a
   message that names it, and a loaded document carries the same invariants
   `normalize()` restores in crates/forge-blocks — never blockless, and
   columns hold no nested columns and no empty cells. */
import { describe, it, expect } from 'vitest';
import { DocumentLoadError, loadDocument } from '../src/load';
import type { Block } from '../src/types';
import { DOC_VERSION } from '../src/types';

const p = (id: string, md = ''): Block => ({ id, type: 'paragraph', md });
const doc = (...blocks: unknown[]): unknown => ({ version: DOC_VERSION, blocks });

describe('version', () => {
  it('accepts the current version', () => {
    expect(loadDocument(doc(p('a'))).version).toBe(DOC_VERSION);
  });
  it('rejects an unrecognised version, naming both versions', () => {
    expect(() => loadDocument({ version: 2, blocks: [] })).toThrowError(/version 2.*version 1/);
  });
  it('rejects a missing or non-numeric version', () => {
    expect(() => loadDocument({ blocks: [] })).toThrowError(/version/);
    expect(() => loadDocument({ version: '1', blocks: [] })).toThrowError(/version/);
  });
  it('refuses the version before looking at the blocks', () => {
    // A future version may change what a block is; the version message is
    // the useful one.
    expect(() => loadDocument({ version: 9, blocks: ['garbage'] })).toThrowError(/version 9/);
  });
});

describe('document shape', () => {
  it('rejects a non-object', () => {
    for (const bad of [null, undefined, 'doc', 7, []]) {
      expect(() => loadDocument(bad)).toThrowError(DocumentLoadError);
    }
  });
  it('rejects a missing or non-array blocks list', () => {
    expect(() => loadDocument({ version: 1 })).toThrowError(/blocks/);
    expect(() => loadDocument({ version: 1, blocks: {} })).toThrowError(/blocks/);
  });
});

describe('malformed blocks', () => {
  it('rejects a non-object block, naming its place', () => {
    expect(() => loadDocument(doc('text'))).toThrowError(/blocks\[0\]/);
  });
  it('rejects a block with no type', () => {
    expect(() => loadDocument(doc({ id: 'a', md: 'x' }))).toThrowError(/blocks\[0\].*type/);
  });
  it('rejects an unknown type, naming it', () => {
    expect(() => loadDocument(doc({ id: 'a', type: 'wibble' }))).toThrowError(/wibble/);
  });
  it('rejects a block with no id', () => {
    expect(() => loadDocument(doc({ type: 'paragraph', md: '' }))).toThrowError(
      /blocks\[0\].*id/,
    );
  });
  it('rejects a missing required field, naming block, type and field', () => {
    expect(() => loadDocument(doc(p('a'), { id: 'b', type: 'heading', level: 1 }))).toThrowError(
      /blocks\[1\] \(heading\).*"md"/,
    );
  });
  it('rejects a field of the wrong shape, saying what it wanted', () => {
    expect(() =>
      loadDocument(doc({ id: 'a', type: 'heading', level: 'one', md: '' })),
    ).toThrowError(/"level".*number/);
    expect(() =>
      loadDocument(doc({ id: 'a', type: 'bar_chart', categories: [], series: 3 })),
    ).toThrowError(/"series".*array/);
  });
  it('throws DocumentLoadError, not a bare Error', () => {
    expect(() => loadDocument(doc({}))).toThrowError(DocumentLoadError);
  });
});

describe('loaded blocks', () => {
  it('keeps ids and fields', () => {
    const loaded = loadDocument(doc(p('a', 'hello')));
    expect(loaded.blocks[0]).toEqual({ id: 'a', type: 'paragraph', md: 'hello' });
  });
  it('drops fields the schema does not declare, as serde does', () => {
    const loaded = loadDocument(doc({ id: 'a', type: 'paragraph', md: '', junk: 1 }));
    expect(loaded.blocks[0]).not.toHaveProperty('junk');
  });
  it('drops a null optional field, as serde reads Option::None', () => {
    const loaded = loadDocument(doc({ id: 'a', type: 'image', src: 's', alt: '', width: null }));
    expect(loaded.blocks[0]).not.toHaveProperty('width');
  });
  it('rejects null for a required field', () => {
    expect(() => loadDocument(doc({ id: 'a', type: 'paragraph', md: null }))).toThrowError(
      /"md"/,
    );
  });
  it('accepts null for a custom payload, which is untyped', () => {
    const loaded = loadDocument(doc({ id: 'a', type: 'custom', kind: 'k', data: null }));
    expect(loaded.blocks[0]).toEqual({ id: 'a', type: 'custom', kind: 'k', data: null });
  });
});

describe('normalisation', () => {
  it('a blockless document gets one empty paragraph', () => {
    const loaded = loadDocument({ version: DOC_VERSION, blocks: [] });
    expect(loaded.blocks).toHaveLength(1);
    expect(loaded.blocks[0]).toMatchObject({ type: 'paragraph', md: '' });
  });
  it('an empty column cell gets an empty paragraph', () => {
    const loaded = loadDocument(
      doc({ id: 'c', type: 'columns', columns: [{ ratio: 1, blocks: [] }] }),
    );
    const cols = loaded.blocks[0] as Extract<Block, { type: 'columns' }>;
    expect(cols.columns[0]!.blocks).toHaveLength(1);
    expect(cols.columns[0]!.blocks[0]).toMatchObject({ type: 'paragraph', md: '' });
  });
  it('a nested columns block is dropped from a cell', () => {
    const nested = { id: 'n', type: 'columns', columns: [{ ratio: 1, blocks: [p('x')] }] };
    const loaded = loadDocument(
      doc({ id: 'c', type: 'columns', columns: [{ ratio: 1, blocks: [nested, p('a')] }] }),
    );
    const cols = loaded.blocks[0] as Extract<Block, { type: 'columns' }>;
    expect(cols.columns[0]!.blocks).toEqual([p('a')]);
  });
  it('a cell emptied by that drop is refilled', () => {
    const nested = { id: 'n', type: 'columns', columns: [{ ratio: 1, blocks: [p('x')] }] };
    const loaded = loadDocument(
      doc({ id: 'c', type: 'columns', columns: [{ ratio: 1, blocks: [nested] }] }),
    );
    const cols = loaded.blocks[0] as Extract<Block, { type: 'columns' }>;
    expect(cols.columns[0]!.blocks).toHaveLength(1);
    expect(cols.columns[0]!.blocks[0]).toMatchObject({ type: 'paragraph', md: '' });
  });
});

describe('columns validation', () => {
  it('rejects a column with no ratio', () => {
    expect(() =>
      loadDocument(doc({ id: 'c', type: 'columns', columns: [{ blocks: [] }] })),
    ).toThrowError(/columns\[0\].*ratio/);
  });
  it('rejects a malformed block inside a cell, naming the whole path', () => {
    expect(() =>
      loadDocument(
        doc({
          id: 'c',
          type: 'columns',
          columns: [{ ratio: 1, blocks: [{ id: 'b', type: 'heading', level: 1 }] }],
        }),
      ),
    ).toThrowError(/blocks\[0\]\.columns\[0\]\.blocks\[0\] \(heading\).*"md"/);
  });
});
