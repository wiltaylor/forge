/* Registry coverage: every block kind the shared crate registers must have a
   web view.

   The kind list is read from `contract/blocks-registry.json` — the generated
   registry the TypeScript side is built from — not from the TypeScript union,
   so a kind added to the registry without a render arm fails here: its
   starter renders to nothing. The render arms are hand-written; nothing else
   checks that they keep up with the registry.

   Two mounts per policy: <BlockRenderer> is the read-only path the gallery
   ships, <BlockEditor> is the editing path the desktop app ships. Both come
   from the package's public interface.

   `docs/web-testing.md` says to query by role and accessible name. This
   suite is a documented exception, like the corpus driver: a coverage sweep
   asks "did this kind render anything at all?", which no role query can
   answer, so it counts element children under the renderer root (`.fbk`)
   and under each editor row (`[data-block-id]`, `.fbk-body`). */
import { describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import { render } from '@solidjs/testing-library';
import { BlockEditor, BlockRenderer, DOC_VERSION, createBlock } from '../src';
import type { Block, BlockDocument, BlockType } from '../src';
import { loadRegistryKinds } from './kinds';

const KINDS = loadRegistryKinds();

describe('the registry', () => {
  it('has kinds to cover', () => {
    expect(KINDS.length).toBeGreaterThan(0);
  });
});

describe('every registry kind renders its starter', () => {
  for (const kind of KINDS) {
    it(`${kind.type} produces an element`, () => {
      const block: Block | undefined = createBlock(kind.type as BlockType);
      // A stale types.gen.ts falls out of its switch with nothing.
      expect(block?.type, `createBlock knows no starter for "${kind.type}"`).toBe(kind.type);
      const { container } = render(() => (
        <BlockRenderer document={{ version: DOC_VERSION, blocks: [block!] }} />
      ));
      const root = container.querySelector('.fbk')!;
      expect(
        root.children.length,
        `no render arm produced an element for "${kind.type}"`,
      ).toBeGreaterThan(0);
    });
  }
});

describe('the editor shows every registry kind', () => {
  it('gives each starter a block row with content', () => {
    const blocks = KINDS.map((k) => createBlock(k.type as BlockType));
    const [doc, setDoc] = createSignal<BlockDocument>({ version: DOC_VERSION, blocks });
    const { container } = render(() => <BlockEditor document={doc()} onChange={setDoc} />);
    for (const block of blocks) {
      const row = container.querySelector(`[data-block-id="${block.id}"]`);
      expect(row, `the editor mounted no row for "${block.type}"`).not.toBeNull();
      const body = row!.querySelector('.fbk-body');
      expect(body, `the row for "${block.type}" has no body`).not.toBeNull();
      expect(
        body!.children.length,
        `the editor shows nothing for "${block.type}"`,
      ).toBeGreaterThan(0);
    }
  });
});
