/* One block row: hover gutter (+ insert, block menu) and the per-type
   editing dispatch — focused text blocks swap in the raw-source textarea,
   everything else gets its dedicated editor or a click-to-focus static view. */
import { Match, Show, Switch, createMemo, createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { DropdownMenu, Select } from '@forge/ui';
import type { MenuItem } from '@forge/ui';
import { CodeEditor, LANGUAGES } from '@forge/code';
import { cloneBlock } from './blockops';
import { useBlocks } from './context';
import { EmojiPopup } from './emojipopup';
import {
  findBlock, insertAfter, moveBlock, removeBlock, replaceBlock, updateBlock, wrapInColumns,
} from './ops';
import { StaticBlock, codeLanguage } from './render';
import { SlashMenu } from './slash';
import { TableEdit } from './tableedit';
import { ColumnsEdit } from './columns';
import { TextBlockEdit } from './textedit';
import type { AdmonitionTone, Block, TextBlock } from './types';
import { createBlock, isTextBlock } from './types';

export interface BlockRowProps {
  block: () => Block;
  num?: number;
  placeholder?: string;
}

export function BlockRow(props: BlockRowProps) {
  const ctx = useBlocks();
  const id = () => props.block().id;
  const focused = () => ctx.focus()?.id === id();
  const [tableCell, setTableCell] = createSignal<{ row: number; col: number } | undefined>();

  const insertBelow = () => {
    const p = createBlock('paragraph');
    ctx.dispatch(insertAfter(ctx.doc(), id(), p));
    ctx.focusBlock(p.id, 0);
  };

  const menuItems = createMemo<MenuItem[]>(() => {
    const b = props.block();
    const inColumn = findBlock(ctx.doc(), id())?.parent.kind === 'column';
    const turnInto = (label: string, make: () => Block): MenuItem => ({
      label,
      onSelect: () => {
        ctx.dispatch(replaceBlock(ctx.doc(), id(), make()));
        ctx.focusBlock(id(), 'end');
      },
    });
    const items: MenuItem[] = [
      {
        label: 'Move up',
        kbd: 'Alt+↑',
        onSelect: () => ctx.dispatch(moveBlock(ctx.doc(), id(), -1)),
      },
      {
        label: 'Move down',
        kbd: 'Alt+↓',
        onSelect: () => ctx.dispatch(moveBlock(ctx.doc(), id(), 1)),
      },
      {
        label: 'Duplicate',
        onSelect: () => ctx.dispatch(insertAfter(ctx.doc(), id(), cloneBlock(props.block()))),
      },
      {
        label: 'Delete',
        danger: true,
        onSelect: () => {
          ctx.blur();
          ctx.dispatch(removeBlock(ctx.doc(), id()));
        },
      },
    ];
    if (isTextBlock(b)) {
      items.push(
        { separator: true },
        turnInto('Turn into text', () => ({ id: id(), type: 'paragraph', md: b.md })),
        turnInto('Turn into heading', () => ({ id: id(), type: 'heading', level: 2, md: b.md })),
        turnInto('Turn into quote', () => ({ id: id(), type: 'quote', md: b.md })),
        turnInto('Turn into callout', () => ({
          id: id(), type: 'admonition', tone: 'info', title: '', md: b.md,
        })),
      );
    }
    if (!inColumn && b.type !== 'columns') {
      items.push(
        { separator: true },
        {
          label: '2 columns',
          onSelect: () => ctx.dispatch(wrapInColumns(ctx.doc(), id(), 2)),
        },
        {
          label: '3 columns',
          onSelect: () => ctx.dispatch(wrapInColumns(ctx.doc(), id(), 3)),
        },
      );
    }
    return items;
  });

  /** Click through a static table lands on the clicked cell. */
  const onTableClick = (e: MouseEvent) => {
    const cell = (e.target as HTMLElement).closest('td,th');
    if (cell) {
      const tr = cell.parentElement as HTMLTableRowElement;
      const inHead = cell.tagName === 'TH';
      setTableCell({
        row: inHead ? -1 : tr.sectionRowIndex,
        col: (cell as HTMLTableCellElement).cellIndex,
      });
    } else {
      setTableCell(undefined);
    }
    ctx.focusBlock(id());
  };

  const staticView = (onClick?: (e: MouseEvent) => void): JSX.Element => (
    <div
      class="fbk-static"
      onClick={(e) => {
        // Todo checkboxes toggle without entering edit mode.
        if ((e.target as HTMLElement).tagName === 'INPUT') return;
        (onClick ?? (() => ctx.focusBlock(id(), 'end')))(e);
      }}
    >
      <StaticBlock block={props.block()} ctx={ctx.render} num={props.num} />
    </div>
  );

  return (
    <div class="fbk-row" classList={{ 'is-focused': focused() }} data-block-id={id()}>
      <div class="fbk-gutter">
        <button type="button" class="fbk-iconbtn" title="Add block below" onClick={insertBelow}>
          +
        </button>
        <DropdownMenu label="⋮⋮" variant="ghost" size="sm" items={menuItems()} />
      </div>
      <div class="fbk-body">
        <Switch fallback={staticView()}>
          <Match when={isTextBlock(props.block()) && props.block().type !== 'admonition'}>
            <Show when={focused()} fallback={staticView()}>
              <TextBlockEdit
                block={props.block as () => TextBlock}
                num={props.num}
                placeholder={props.placeholder}
              />
            </Show>
          </Match>
          <Match when={props.block().type === 'admonition'}>
            <Show when={focused()} fallback={staticView()}>
              <AdmonitionEdit block={props.block as () => AdmonitionBlock} />
            </Show>
          </Match>
          <Match when={props.block().type === 'code' && props.block()}>
            {(b) => <CodeBlockEdit block={b as () => CodeBlock} />}
          </Match>
          <Match when={props.block().type === 'table'}>
            <Show when={focused()} fallback={staticView(onTableClick)}>
              <TableEdit block={props.block as () => TableBlock} initialCell={tableCell()} />
            </Show>
          </Match>
          <Match when={props.block().type === 'columns' && props.block()}>
            {(b) => <ColumnsEdit block={b as () => ColumnsBlock} />}
          </Match>
          <Match when={props.block().type === 'divider'}>
            <div
              class="fbk-static fbk-divider-hit"
              tabindex="0"
              onClick={() => ctx.focusBlock(id())}
              onKeyDown={(e) => {
                if (e.key === 'Backspace' || e.key === 'Delete') {
                  e.preventDefault();
                  ctx.dispatch(removeBlock(ctx.doc(), id()));
                }
              }}
            >
              <hr class="fbk-hr" />
            </div>
          </Match>
          <Match when={props.block().type === 'custom' && props.block()}>
            {(b) => <CustomEdit block={b as () => CustomBlock} focused={focused()} />}
          </Match>
        </Switch>
        <SlashMenu blockId={id()} />
        <EmojiPopup blockId={id()} />
      </div>
    </div>
  );
}

type AdmonitionBlock = Extract<Block, { type: 'admonition' }>;
type CodeBlock = Extract<Block, { type: 'code' }>;
type TableBlock = Extract<Block, { type: 'table' }>;
type ColumnsBlock = Extract<Block, { type: 'columns' }>;
type CustomBlock = Extract<Block, { type: 'custom' }>;

const TONES: { value: AdmonitionTone; label: string }[] = [
  { value: 'info', label: 'Info' },
  { value: 'success', label: 'Success' },
  { value: 'warning', label: 'Warning' },
  { value: 'danger', label: 'Danger' },
];

function AdmonitionEdit(props: { block: () => AdmonitionBlock }) {
  const ctx = useBlocks();
  const id = () => props.block().id;
  return (
    <div class={`fbk-admedit fbk-admedit-${props.block().tone}`}>
      <div class="fbk-admhead">
        <Select
          options={TONES}
          value={props.block().tone}
          onChange={(tone) => ctx.dispatch(updateBlock(ctx.doc(), id(), { tone }))}
        />
        <input
          class="fbk-admtitle"
          placeholder="Title"
          value={props.block().title}
          onInput={(e) => ctx.dispatch(updateBlock(ctx.doc(), id(), { title: e.currentTarget.value }))}
        />
      </div>
      <TextBlockEdit block={props.block} />
    </div>
  );
}

const LANG_OPTIONS = [
  { value: '', label: 'plain' },
  ...Object.keys(LANGUAGES).map((l) => ({ value: l, label: l })),
];

function CodeBlockEdit(props: { block: () => CodeBlock }) {
  const ctx = useBlocks();
  const id = () => props.block().id;
  return (
    <div
      class="fbk-code fbk-codeedit"
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault();
          (document.activeElement as HTMLElement | null)?.blur();
          ctx.blur();
        }
      }}
    >
      <div class="fbk-codehead">
        <Select
          options={LANG_OPTIONS}
          value={codeLanguage(props.block().lang) ?? ''}
          onChange={(lang) => ctx.dispatch(updateBlock(ctx.doc(), id(), { lang }))}
        />
      </div>
      <CodeEditor
        value={props.block().code}
        onChange={(code) => ctx.dispatch(updateBlock(ctx.doc(), id(), { code }))}
        language={codeLanguage(props.block().lang)}
        lineNumbers={false}
        height="auto"
      />
    </div>
  );
}

function CustomEdit(props: { block: () => CustomBlock; focused: boolean }) {
  const ctx = useBlocks();
  const id = () => props.block().id;
  const def = () => ctx.customBlocks()[props.block().kind];
  return (
    <div class="fbk-custom" onClick={() => !props.focused && ctx.focusBlock(id())}>
      <Show
        when={props.focused && def()?.edit}
        fallback={<StaticBlock block={props.block()} ctx={ctx.render} />}
      >
        {(edit) =>
          edit()({
            data: props.block().data,
            onChange: (data) => ctx.dispatch(updateBlock(ctx.doc(), id(), { data })),
          })
        }
      </Show>
    </div>
  );
}
