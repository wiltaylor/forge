/* Read-only block rendering — used standalone via <BlockRenderer> and inside
   the editor for unfocused blocks. Dispatch per type follows the kanban
   fields.tsx Switch/Match idiom. */
import { For, Match, Show, Switch, createMemo } from 'solid-js';
import type { JSX } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { Alert } from '@forge/ui';
import { CodeEditor } from '@forge/code';
import type { LanguageInput } from '@forge/code';
import { InlineMd } from './inline';
import type { Block, BlockDef, BlockDocument } from './types';

export interface RenderCtx {
  customBlocks?: Record<string, BlockDef>;
  emoji?: Record<string, string>;
  linkTarget?: '_blank' | '_self';
  /** Toggle handler for todo checkboxes; absent = disabled boxes. */
  onToggleTodo?: (id: string, checked: boolean) => void;
}

export interface BlockRendererProps extends RenderCtx {
  document: BlockDocument;
  class?: string;
}

/** Read-only renderer for a whole document. */
export function BlockRenderer(props: BlockRendererProps) {
  return (
    <div class={`fbk ${props.class ?? ''}`}>
      <BlockList blocks={props.document.blocks} ctx={props} />
    </div>
  );
}

export function BlockList(props: { blocks: Block[]; ctx: RenderCtx }) {
  const numbers = createMemo(() => listNumbers(props.blocks));
  return (
    <For each={props.blocks}>
      {(b) => <StaticBlock block={b} ctx={props.ctx} num={numbers().get(b.id)} />}
    </For>
  );
}

/** Ordinal per `number`-style list item; runs reset at non-list blocks. */
export function listNumbers(blocks: Block[]): Map<string, number> {
  const map = new Map<string, number>();
  let n = 0;
  for (const b of blocks) {
    if (b.type === 'list_item' && b.style === 'number') map.set(b.id, ++n);
    else if (b.type !== 'list_item') n = 0;
  }
  return map;
}

const CODE_LANGS = new Set(['js', 'jsx', 'ts', 'tsx', 'python', 'json', 'css', 'html', 'shell']);

/** Aliases → @forge/code language names; unknown languages render plain. */
export function codeLanguage(lang: string): LanguageInput | undefined {
  const alias: Record<string, string> = {
    javascript: 'js', typescript: 'ts', py: 'python', sh: 'shell', bash: 'shell', zsh: 'shell',
  };
  const name = alias[lang.toLowerCase()] ?? lang.toLowerCase();
  return CODE_LANGS.has(name) ? (name as LanguageInput) : undefined;
}

export function StaticBlock(props: { block: Block; ctx: RenderCtx; num?: number }): JSX.Element {
  const inline = (md: string) => (
    <InlineMd md={md} emoji={props.ctx.emoji} linkTarget={props.ctx.linkTarget} />
  );
  return (
    <Switch>
      <Match when={props.block.type === 'paragraph' && props.block}>
        {(b) => <p class="fbk-p">{inline(b().md)}</p>}
      </Match>
      <Match when={props.block.type === 'heading' && props.block}>
        {(b) => (
          <Dynamic component={`h${b().level}`} class={`fbk-h fbk-h${b().level}`}>
            {inline(b().md)}
          </Dynamic>
        )}
      </Match>
      <Match when={props.block.type === 'list_item' && props.block}>
        {(b) => (
          <div class="fbk-li" style={{ '--fbk-indent': b().indent }}>
            <span class="fbk-li-marker">
              <Show
                when={b().style === 'todo'}
                fallback={b().style === 'number' ? `${props.num ?? 1}.` : '•'}
              >
                <input
                  type="checkbox"
                  checked={b().checked}
                  disabled={!props.ctx.onToggleTodo}
                  onChange={(e) => props.ctx.onToggleTodo?.(b().id, e.currentTarget.checked)}
                />
              </Show>
            </span>
            <span classList={{ 'fbk-li-done': b().style === 'todo' && b().checked }}>
              {inline(b().md)}
            </span>
          </div>
        )}
      </Match>
      <Match when={props.block.type === 'quote' && props.block}>
        {(b) => <blockquote class="fbk-quote">{inline(b().md)}</blockquote>}
      </Match>
      <Match when={props.block.type === 'divider'}>
        <hr class="fbk-hr" />
      </Match>
      <Match when={props.block.type === 'code' && props.block}>
        {(b) => (
          <div class="fbk-code" data-lang={b().lang || undefined}>
            <CodeEditor
              value={b().code}
              language={codeLanguage(b().lang)}
              readOnly
              lineNumbers={false}
              height="auto"
            />
          </div>
        )}
      </Match>
      <Match when={props.block.type === 'table' && props.block}>
        {(b) => (
          <div class="fbk-tablewrap">
            <table class="fbk-table">
              <thead>
                <tr>
                  <For each={b().header}>{(cell) => <th>{inline(cell)}</th>}</For>
                </tr>
              </thead>
              <tbody>
                <For each={b().rows}>
                  {(row) => (
                    <tr>
                      <For each={row}>{(cell) => <td>{inline(cell)}</td>}</For>
                    </tr>
                  )}
                </For>
              </tbody>
            </table>
          </div>
        )}
      </Match>
      <Match when={props.block.type === 'admonition' && props.block}>
        {(b) => (
          <div class="fbk-adm">
            <Alert tone={b().tone} title={b().title ? inline(b().title) : undefined}>
              {inline(b().md)}
            </Alert>
          </div>
        )}
      </Match>
      <Match when={props.block.type === 'columns' && props.block}>
        {(b) => (
          <div class="fbk-cols">
            <For each={b().columns}>
              {(col) => (
                <div class="fbk-col" style={{ 'flex-grow': col.ratio * 1000 }}>
                  <BlockList blocks={col.blocks} ctx={props.ctx} />
                </div>
              )}
            </For>
          </div>
        )}
      </Match>
      <Match when={props.block.type === 'custom' && props.block}>
        {(b) => <CustomBlockView block={b()} ctx={props.ctx} />}
      </Match>
    </Switch>
  );
}

function CustomBlockView(props: {
  block: Extract<Block, { type: 'custom' }>;
  ctx: RenderCtx;
}) {
  const def = createMemo(() => props.ctx.customBlocks?.[props.block.kind]);
  return (
    <Show
      when={def()}
      fallback={
        <Alert tone="warning" title={`Unknown block “${props.block.kind}”`}>
          Register a BlockDef for this kind to render it.
        </Alert>
      }
    >
      {(d) => <div class="fbk-custom">{d().render({ data: props.block.data })}</div>}
    </Show>
  );
}
