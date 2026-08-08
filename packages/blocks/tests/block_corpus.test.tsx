/* The block key corpus driven against the web block editor.

   The corpus (`contract/blocks/corpus.json`) is authored data: a starting
   document, an address, a key sequence, and the document that must result.
   The Rust kits run it through `crates/forge-block-corpus`; this driver is the
   web kit's adapter — it mounts <BlockEditor>, puts it at the case's address
   in the case's mode, dispatches each key as a real DOM event, and compares
   the document that comes back out of `onChange`.

   The web kit cannot call the Rust editing policy, so this file is what keeps
   its third implementation honest: both languages press the same keys on the
   same documents, and a divergence in either fails a test. */
import { describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import { fireEvent, render } from '@solidjs/testing-library';
import { BlockEditor } from '../src/editor';
import type { Block, BlockDocument } from '../src/types';
import type { Case, Key } from './corpus';
import {
  WEB, caretIndex, caseDocument, caseExpected, judged, keyLabel, loadCorpus, webVerdict,
} from './corpus';

const corpus = loadCorpus();

/* ---------------- keys --------------------------------------------------- */

/** `KeyboardEvent.code` → the `KeyboardEvent.key` a browser reports for it.
    Only the named keys need a table: a printable key carries the character it
    produces in the case itself, because that is what a focused field reads. */
const NAMED_KEYS: Record<string, string> = {
  Enter: 'Enter',
  NumpadEnter: 'Enter',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Tab: 'Tab',
  Escape: 'Escape',
  Space: ' ',
  ArrowUp: 'ArrowUp',
  ArrowDown: 'ArrowDown',
  ArrowLeft: 'ArrowLeft',
  ArrowRight: 'ArrowRight',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
};

function keyName(key: Key): string {
  if (key.key !== undefined) return key.key;
  const named = NAMED_KEYS[key.code];
  // A code with no key is a hard error, never a skip: a corpus that could
  // silently drop a key would be a corpus that passes by not running.
  if (named === undefined) throw new Error(`no browser key for code ${JSON.stringify(key.code)}`);
  return named;
}

/* ---------------- the driver --------------------------------------------- */

/** A text field of the editor: the block textarea or a table cell input. */
type Field = HTMLTextAreaElement | HTMLInputElement;

/** Mount the editor, put it at the case's address, press the case's keys, and
    hand back the document the editor ended on. */
async function drive(c: Case): Promise<BlockDocument> {
  const [doc, setDoc] = createSignal(caseDocument(c));
  const { container } = render(() => <BlockEditor document={doc()} onChange={setDoc} />);

  await focusAddress(container, doc(), c);
  for (const key of c.keys) await press(c, key);

  return doc();
}

/** Click the block the case addresses, then take up the case's mode. Clicking
    is how a user reaches a block, so the address is set through the editor
    rather than around it.

    `docs/web-testing.md` says to query by role and accessible name. This
    driver is the documented exception, because a case addresses a block by
    *position* — root block 2, cell (1, 0) — and a document of eight paragraphs
    gives eight identical textboxes. The Rust drivers address the same way, by
    `Address`. So the queries here are positional, and the price is that a
    markup change moves them: `[data-block-id]` on the row, `.fbk-static` for
    the unfocused view, and `data-row`/`data-col` on a table cell. */
async function focusAddress(container: HTMLElement, doc: BlockDocument, c: Case): Promise<void> {
  const id = addressedId(doc, c);
  const row = container.querySelector<HTMLElement>(`[data-block-id="${id}"]`);
  if (!row) throw new Error(`${c.id}: no block row at ${JSON.stringify(c.at)}`);
  const view = row.querySelector<HTMLElement>('.fbk-static');
  if (!view) throw new Error(`${c.id}: the block at ${JSON.stringify(c.at)} has no static view`);
  fireEvent.click(view);
  await flush();

  if (c.at.row !== undefined && c.at.col !== undefined) {
    // Display row 0 is the header, which the web kit numbers -1.
    const cell = row.querySelector<HTMLInputElement>(
      `input.fbk-cell[data-row="${c.at.row - 1}"][data-col="${c.at.col}"]`,
    );
    if (!cell) throw new Error(`${c.id}: no table cell at row ${c.at.row}, col ${c.at.col}`);
    place(cell, cell.value.length);
    return;
  }

  if (c.at.caret === undefined) {
    // Block-selected: no caret, so the block's own view takes the keys.
    view.focus();
    return;
  }
  const ta = row.querySelector<HTMLTextAreaElement>('textarea');
  if (!ta) throw new Error(`${c.id}: the block at ${JSON.stringify(c.at)} takes no text caret`);
  place(ta, caretIndex(ta.value, c.at.caret));
}

/** The id of the block the case addresses. */
function addressedId(doc: BlockDocument, c: Case): string {
  const root = doc.blocks[c.at.block];
  if (!root) throw new Error(`${c.id}: the document has no block ${c.at.block}`);
  if (c.at.column === undefined || c.at.index === undefined) return root.id;
  if (root.type !== 'columns')
    throw new Error(`${c.id}: block ${c.at.block} is not a columns block`);
  const cell: Block | undefined = root.columns[c.at.column]?.blocks[c.at.index];
  if (!cell)
    throw new Error(`${c.id}: no block ${c.at.index} in column ${c.at.column}`);
  return cell.id;
}

function place(field: Field, caret: number): void {
  field.focus();
  field.setSelectionRange(caret, caret);
}

/** Press one key on whatever the editor has focused, the way a browser does:
    the keydown first, and the character only if nothing took the key.

    Nothing focused is a hard error, never a quiet no-op: a driver that pressed
    keys at the document would be a driver that passes by not running. */
async function press(c: Case, key: Key): Promise<void> {
  const target = document.activeElement as HTMLElement | null;
  if (!target || target === document.body)
    throw new Error(`${c.id}: the editor has nothing focused for ${keyLabel(key)}`);

  const notTaken = fireEvent.keyDown(target, {
    key: keyName(key),
    code: key.code,
    shiftKey: key.shift ?? false,
    ctrlKey: key.ctrl ?? false,
    altKey: key.alt ?? false,
  });
  if (notTaken && key.key !== undefined && isField(target)) typeInto(target, key.key);
  await flush();
}

function isField(el: HTMLElement): el is Field {
  return el instanceof HTMLTextAreaElement || el instanceof HTMLInputElement;
}

/** Insert the produced character at the caret and report the input, which is
    what a focused text field does. */
function typeInto(field: Field, text: string): void {
  const start = field.selectionStart ?? field.value.length;
  const end = field.selectionEnd ?? start;
  field.value = field.value.slice(0, start) + text + field.value.slice(end);
  field.setSelectionRange(start + text.length, start + text.length);
  fireEvent.input(field);
}

/** Let the editor's deferred focus placement run. Solid renders synchronously,
    but <BlockEditor> places the caret in a microtask (an animation frame when
    the field is not mounted yet). */
async function flush(): Promise<void> {
  await Promise.resolve();
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => requestAnimationFrame(() => resolve(null)));
}

/* ---------------- the run ------------------------------------------------- */

describe('the block key corpus', () => {
  it('has cases for the web kit', () => {
    const running = corpus.cases.filter((c) => webVerdict(c) !== 'skip');
    expect(running.length).toBeGreaterThan(0);
  });

  for (const c of corpus.cases) {
    const verdict = webVerdict(c);
    if (verdict === 'skip') continue;

    it(`${c.id}: ${c.title}`, async () => {
      const actual = judged(await drive(c));
      const expected = judged(caseExpected(c));
      const keys = c.keys.map(keyLabel).join(', ');

      if (verdict === 'match') {
        expect(actual, `keys: ${keys}`).toEqual(expected);
        return;
      }
      // A recorded divergence is not a skip: the keys are pressed and the
      // result must still be wrong, so closing the gap fails this test until
      // the note goes with it.
      const d = c.diverges![WEB]!;
      expect(
        actual,
        `keys: ${keys}\nthe corpus records this as a known web divergence closed by ` +
          `#${d.issue}, but the web kit now matches it. Drop the \`diverges\` note ` +
          `from the case.\nrecorded: ${d.why}`,
      ).not.toEqual(expected);
    });
  }
});
