/* Tool-call boxes must keep the expand/collapse state the user chose when the
   transcript updates underneath them — a poll that appends one record rebuilds
   every row (`entries()` hands <For> fresh objects) and a patched message
   replaces its own blocks, so anything held inside a box is lost. */
import { afterEach, describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { render } from 'solid-js/web';
import { ChatToolCall } from '../src/toolcall';
import { ChatView } from '../src/view';
import type { ChatItem, ChatToolCallData } from '../src/types';

const PARTICIPANTS = [
  { id: 'user', name: 'you' },
  { id: 'agent', name: 'agent' },
];

let dispose: (() => void) | undefined;
let host: HTMLDivElement | undefined;

afterEach(() => {
  dispose?.();
  host?.remove();
  dispose = undefined;
  host = undefined;
});

function mount(code: () => JSX.Element): HTMLDivElement {
  host = document.createElement('div');
  document.body.append(host);
  dispose = render(code, host);
  return host;
}

function toolItem(id: string, tool: ChatToolCallData): ChatItem {
  return { type: 'message', id, author: 'agent', blocks: [{ kind: 'tool', tool }] };
}

const call = (key: string, extra?: Partial<ChatToolCallData>): ChatToolCallData => ({
  key,
  name: key,
  status: 'success',
  result: `result of ${key}`,
  ...extra,
});

const heads = (root: HTMLElement) =>
  Array.from(root.querySelectorAll<HTMLButtonElement>('.fchat-tool-head'));
const headFor = (root: HTMLElement, name: string) =>
  heads(root).find((head) => head.querySelector('.fchat-tool-name')?.textContent === name)!;
const isOpen = (head: HTMLButtonElement) => head.getAttribute('aria-expanded') === 'true';

describe('ChatView tool-call expansion', () => {
  it('keeps a box expanded when a record is appended', () => {
    const [items, setItems] = createSignal<ChatItem[]>([toolItem('ev-1', call('a'))]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    const before = headFor(root, 'a');
    expect(isOpen(before)).toBe(false);
    before.click();
    expect(isOpen(headFor(root, 'a'))).toBe(true);

    setItems((prev) => [...prev, toolItem('ev-2', call('b'))]);

    const after = headFor(root, 'a');
    // The row really was rebuilt — otherwise this test proves nothing.
    expect(after).not.toBe(before);
    expect(isOpen(after)).toBe(true);
  });

  it('keeps a box collapsed when the user closed it', () => {
    const [items, setItems] = createSignal<ChatItem[]>([
      toolItem('ev-1', call('a', { defaultOpen: true })),
    ]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    expect(isOpen(headFor(root, 'a'))).toBe(true);
    headFor(root, 'a').click();
    expect(isOpen(headFor(root, 'a'))).toBe(false);

    setItems((prev) => [...prev, toolItem('ev-2', call('b'))]);
    expect(isOpen(headFor(root, 'a'))).toBe(false);
  });

  it('leaves untouched boxes on defaultOpen', () => {
    const [items, setItems] = createSignal<ChatItem[]>([
      toolItem('ev-1', call('open', { defaultOpen: true })),
      toolItem('ev-2', call('shut')),
    ]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    setItems((prev) => [...prev, toolItem('ev-3', call('later'))]);

    expect(isOpen(headFor(root, 'open'))).toBe(true);
    expect(isOpen(headFor(root, 'shut'))).toBe(false);
    expect(isOpen(headFor(root, 'later'))).toBe(false);
  });

  it('toggles one box without touching its neighbours', () => {
    const [items, setItems] = createSignal<ChatItem[]>([
      toolItem('ev-1', call('a')),
      toolItem('ev-2', call('b')),
    ]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    headFor(root, 'a').click();
    setItems((prev) => [...prev, toolItem('ev-3', call('c'))]);

    expect(isOpen(headFor(root, 'a'))).toBe(true);
    expect(isOpen(headFor(root, 'b'))).toBe(false);
    expect(isOpen(headFor(root, 'c'))).toBe(false);
  });

  it('keeps state through the update that patches the call in place', () => {
    // What `transcript.ts` does when the result lands: a new tool object, a
    // new blocks array and a new message object, at the same key.
    const [items, setItems] = createSignal<ChatItem[]>([
      toolItem('ev-1', { key: 'a', name: 'a', status: 'running', args: 'x' }),
    ]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    headFor(root, 'a').click();
    setItems(() => [
      toolItem('ev-1', { key: 'a', name: 'a', status: 'success', args: 'x', result: 'done' }),
    ]);

    expect(isOpen(headFor(root, 'a'))).toBe(true);
    expect(root.querySelector('.fchat-tool-body')?.textContent).toContain('done');
  });

  it('tracks nested child calls independently', () => {
    const nested = call('parent', {
      children: [
        { name: 'child-0', status: 'success', result: '0' },
        { name: 'child-1', status: 'success', result: '1' },
      ],
    });
    const [items, setItems] = createSignal<ChatItem[]>([toolItem('ev-1', nested)]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    headFor(root, 'parent').click();
    headFor(root, 'child-0').click();
    setItems((prev) => [...prev, toolItem('ev-2', call('other'))]);

    expect(isOpen(headFor(root, 'parent'))).toBe(true);
    expect(isOpen(headFor(root, 'child-0'))).toBe(true);
    expect(isOpen(headFor(root, 'child-1'))).toBe(false);
  });

  it('gives each ChatView its own registry', () => {
    const items: ChatItem[] = [toolItem('ev-1', call('a'))];
    const root = mount(() => (
      <>
        <div class="pane-1">
          <ChatView items={items} participants={PARTICIPANTS} self="user" variant="room" />
        </div>
        <div class="pane-2">
          <ChatView items={items} participants={PARTICIPANTS} self="user" variant="room" />
        </div>
      </>
    ));

    const pane = (n: number) => root.querySelector<HTMLElement>(`.pane-${n}`)!;
    headFor(pane(1), 'a').click();

    expect(isOpen(headFor(pane(1), 'a'))).toBe(true);
    expect(isOpen(headFor(pane(2), 'a'))).toBe(false);
  });

  it('falls back to local state for an unkeyed box', () => {
    const [items, setItems] = createSignal<ChatItem[]>([
      toolItem('ev-1', { name: 'nokey', status: 'success', result: 'r' }),
    ]);
    const root = mount(() => (
      <ChatView items={items()} participants={PARTICIPANTS} self="user" variant="room" />
    ));

    headFor(root, 'nokey').click();
    expect(isOpen(headFor(root, 'nokey'))).toBe(true);
    // No key, nothing to remember it by: the pre-existing behaviour stands.
    setItems((prev) => [...prev, toolItem('ev-2', call('b'))]);
    expect(isOpen(headFor(root, 'nokey'))).toBe(false);
  });
});

describe('ChatToolCall standalone', () => {
  it('holds its own state with no open/onToggle', () => {
    const root = mount(() => (
      <ChatToolCall tool={{ name: 'solo', status: 'success', result: 'r' }} />
    ));

    const head = heads(root)[0]!;
    expect(isOpen(head)).toBe(false);
    head.click();
    expect(isOpen(head)).toBe(true);
    head.click();
    expect(isOpen(head)).toBe(false);
  });

  it('honours defaultOpen with no open/onToggle', () => {
    const root = mount(() => (
      <ChatToolCall tool={{ name: 'solo', status: 'success', result: 'r', defaultOpen: true }} />
    ));
    expect(isOpen(heads(root)[0]!)).toBe(true);
  });

  it('is controlled when given open and onToggle', () => {
    const [open, setOpen] = createSignal(false);
    const seen: boolean[] = [];
    const root = mount(() => (
      <ChatToolCall
        tool={{ name: 'solo', status: 'success', result: 'r' }}
        open={open()}
        onToggle={(next) => seen.push(next)}
      />
    ));

    const head = heads(root)[0]!;
    head.click();
    // The owner decides: no prop change, no visual change.
    expect(seen).toEqual([true]);
    expect(isOpen(head)).toBe(false);
    setOpen(true);
    expect(isOpen(head)).toBe(true);
  });
});
