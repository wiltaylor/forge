# The block key corpus

`corpus.json` is the block editor's keyboard model as authored data. Each case
states a starting document, where the editor is, which keys are pressed, and
the document that must result. Three editors implement that model — the
ratatui kit, the egui kit and the web kit — and this file is what keeps them
saying the same thing.

A **driver** reads the corpus, puts its kit's editor at the case's address in
the case's mode, presses the keys, and compares the document it gets back.
Adding a case here covers every driver at once.

| Kit id | Driver | Status |
|---|---|---|
| `rust-tui` | `crates/forge-tui/tests/block_corpus.rs` | landed |
| `rust-egui` | `crates/forge-egui/tests/block_corpus.rs` | landed |
| `web` | `packages/blocks/tests/block_corpus.test.tsx` | landed |

The Rust drivers share the loader, the comparison and the runner:
`crates/forge-block-corpus`. It knows nothing about an editor. The web driver
reads the same file through `packages/blocks/tests/corpus.ts`, which is that
crate's shape in TypeScript. The two readings cannot be one module. That is the
reason the corpus is a file and not shared code.

The web driver mounts `<BlockEditor>` in a DOM, clicks the block the case
addresses, and dispatches each key as a real event. It reads the document back
from `onChange`, so it asserts on what the editor tells its owner.

`just block-corpus-test` runs every driver.

## Why a corpus and not shared code

The editing policy lives in `forge-blocks`: `resolve_key` turns a keypress, an
address and a document into the operation to perform, and both Rust kits adapt
their own key type onto the shared key shape — the ratatui kit in issue #31,
the egui kit in #32. The web kit cannot call Rust, so it keeps its own
implementation. This file is how that third implementation stays honest: both
languages run the same table, and a divergence in either fails a test.

## A case asserts on documents

```json
{
  "id": "unique-kebab-case",
  "title": "one line, present tense",
  "note": "optional; why the case is written this way",
  "applies": ["rust-tui", "rust-egui", "web"],
  "inapplicable": {},
  "diverges": {},
  "doc": [ <block>, ... ],
  "at": { "block": 1, "caret": 0 },
  "keys": [ <key>, ... ],
  "expect": [ <block>, ... ]
}
```

The document is what matters. A test that asserted on cursor bookkeeping, or on
which internal mode the editor entered, would break on every refactor and tell
you nothing about correctness — so a case never states either.

`doc` and `expect` are block lists in the frozen wire shape
(`docs/api-contract.md` has the envelope; `crates/forge-blocks/src/schema.rs`
has the blocks), with one omission: **no `id`**. Block identity is editor
bookkeeping, not editing policy, and a freshly split block gets a fresh id, so
the comparison drops every id on both sides. Nested blocks inside a `columns`
cell drop theirs too.

The corpus is **authored input, not recorded output**. A case says what the
editors must do. When a case and an editor disagree, one of them is wrong, and
which one is a judgement call — not something the corpus regenerates away.

## Where the editor starts

`at` addresses one block and picks a mode:

| `at` | Means |
|---|---|
| `{"block": 2}` | Root block 2, block-selected: structural keys, no text caret. |
| `{"block": 2, "caret": 0}` | Root block 2, text caret at byte offset 0. |
| `{"block": 0, "row": 1, "col": 0}` | The table at root block 0, editing one cell. Display row 0 is the header, body rows follow. |
| `{"block": 0, "column": 1, "index": 0, "caret": 3}` | Block 0 of column 1 of the `columns` block at root 0. |

`caret` is a **byte** offset into the block's markdown source, the same unit
`forge_blocks::split` takes.

## Keys

A key is one press in the browser `KeyboardEvent` vocabulary — the same
layout-independent `code` (plus the produced character in `key`) this repo
already uses for its remote-protocol keymaps, in
`crates/forge-core/src/widgets/keymap`:

```json
{ "code": "Tab", "shift": true }
{ "code": "Digit3", "key": "#", "shift": true }
{ "code": "ArrowDown", "alt": true }
```

`code` is required. `key` carries the produced character for printables, and a
driver types it rather than synthesising a key event — that is what a focused
text field reads. `shift`, `ctrl` and `alt` default to false.

That shape is `forge_blocks::Key`, the one `resolve_key` reads, so a Rust kit
adapts its key type once and speaks to the corpus and the resolver with it.

Each driver adapts `code` to its own key type. A code the driver has no key
for is a hard error, not a skip: a corpus that could silently drop a key would
be a corpus that passes by not running.

## Applicability, and why it is not optional

Every case names **every** kit, in exactly one of `applies`, `inapplicable` or
`diverges`. `Corpus::validate` rejects a case that leaves one out, so a gap
cannot be created by forgetting. A gap has to be written down, with a reason.

**`inapplicable`** states what the kit *cannot* do:

```json
"inapplicable": {
  "rust-egui": "Column ops in the egui kit are toolbar buttons under the grid; the kit binds no key to press."
}
```

"Not implemented yet" is not a reason. A case a kit could serve stays in
`applies` and fails until it does. A conditional skip inside a driver is never
correct: it turns a real divergence into a green run.

**`diverges`** states that the kit produces something *other* than `expect`
today, and names the issue that closes it:

```json
"diverges": {
  "rust-tui": {
    "issue": 28,
    "why": "The ratatui kit's built-in palette lists Heading 1 to 3 only, so the query has no match and Enter picks nothing."
  }
}
```

This is not a skip. The runner presses the keys and asserts the result is
**still wrong**. Close the gap and the run turns red — "this now passes, drop
the note" — so a stale divergence cannot outlive the fix that ended it.

## The four known divergences — closed

The spec (#3) records four ways the three kits had already drifted apart.
Issue #28 closed them. Three were keypress behaviour and are cases here; each
now lists every kit in `applies` and carries no `diverges` note:

| Divergence | Case | Kit that differed |
|---|---|---|
| A heading level offered in two kits but not the third | `slash-palette-offers-a-heading-4` | `rust-tui` |
| A table starter three-by-two in two kits and two-by-one in the third | `slash-palette-starts-a-three-by-two-table` | `rust-tui` |
| A forward-merge with no key binding in one kit | `delete-at-the-end-merges-the-next-paragraph-forward` | `rust-egui` |

Spec #3 names the *web* kit as the one missing the forward merge. It is not:
`packages/blocks/src/textedit.tsx` handles Delete at the end of a block, and
the egui kit's text-mode key table had no Delete arm at all. The corpus
recorded the kit that differed, so #28 fixed the right one.

The fourth — a chart kind missing from one kit's painted-output snapshot list —
was a test fixture, not a keypress. `slash-palette-offers-the-line-chart-kind`
covers the behaviour a key corpus can state (the palette entry and the
registry's starter payload); #28 filled the snapshot list in.

Where the kits differed, the corpus states the **majority** behaviour, as the
spec decided.

The register is empty now. That is not a permanent state: a newly found
divergence gets a `diverges` note naming the issue that will close it, and
`the_known_divergences_are_recorded` in `crates/forge-block-corpus/tests/`
is where an unexpected one gets noticed.

## What the corpus covers

- The demote-before-merge rule, backwards and forwards: only paragraphs merge
  into other blocks, so a heading, quote, list item or admonition demotes first
  and merges on the next press.
- Indent clamping: 0 at the bottom, 5 at the top, and non-list blocks ignoring
  Tab entirely.
- The whole line-start shortcut grammar — every heading level, both bullet
  markers, both numbered markers, the todo prefixes, quote, code fence,
  divider, math and each callout tone — plus the two ways it must *not* fire:
  mid-block, and on a block that is not a paragraph.
- The table operations that a key reaches: typing through to a cell, Enter
  moving down and appending, Ctrl+Enter inserting, Tab and Shift+Tab between
  cells, and the ratatui kit's column keys.
- Splitting, list continuation, block moves, and editing inside a column cell.

## What the corpus replaced

Both languages used to assert this model by hand, in two suites that had drifted
into near-duplicates of each other: `crates/forge-blocks/tests/ops.rs` and
`packages/blocks/tests/ops.test.ts`. Everything the corpus states is gone from
both, so a rule is authored once and adding a case covers both languages.

What stayed in those suites is what no key reaches: column wrapping and ratios,
row removal, the identity discipline the web editor's rendering depends on, the
shortcut spellings that must *not* convert, and the markdown conversion.
