import { For, Match, Show, Switch, createEffect, createMemo, createSignal, mergeProps, onCleanup, onMount } from 'solid-js';
import type { JSX } from 'solid-js';
import { Avatar } from '@forge/ui';
import type {
  ChatDividerData,
  ChatEventData,
  ChatItem,
  ChatMessageData,
  ChatParticipant,
  LinkResolver,
} from './types';
import { ChatMessage } from './message';
import { dayKey, formatDay, formatTime, isoTime } from './internal/time';
import { ArrowDownSvg } from './internal/icons';

/* ---------------- ChatView ---------------------------------------------------- */
/* Data-driven transcript. Owns message grouping, day dividers, the unread
   marker, typing row, and scroll behavior: pinned-to-bottom while the user is
   at the bottom, a "N new messages" jump pill when they've scrolled back, and
   scroll compensation when history is prepended via onReachTop. */
export interface ChatViewProps {
  items: ChatItem[];
  participants: ChatParticipant[];
  /** Participant id rendered as the "own" (right-aligned) side. */
  self: string;
  /** 'direct' hides names (1:1); 'room' shows avatar + name for everyone. */
  variant?: 'direct' | 'room';
  /** Participant ids currently typing. */
  typing?: string[];
  /** Item id after which the "New" marker renders. */
  unreadAfter?: string;
  /** Max minutes between messages of one group (default 5). */
  groupWindow?: number;
  dayDividers?: boolean;
  showTimes?: boolean;
  /** Server-backed metadata resolver for link blocks without `meta`. */
  resolveLink?: LinkResolver;
  /** Fires near the top of scrollback — prepend older items, scroll is compensated. */
  onReachTop?: () => void;
  /** Render text blocks as markdown (default true). */
  markdown?: boolean;
  class?: string;
  style?: JSX.CSSProperties;
  /** Rendered below the scroll area — the ChatComposer slot. */
  children?: JSX.Element;
}

type Entry =
  | { k: 'day'; id: string; label: string }
  | { k: 'unread'; id: string }
  | { k: 'event'; id: string; item: ChatEventData }
  | { k: 'divider'; id: string; item: ChatDividerData }
  | { k: 'group'; id: string; author: string; at?: ChatMessageData['at']; messages: ChatMessageData[] };

const NEAR_BOTTOM = 48;
const NEAR_TOP = 64;

export function ChatView(props: ChatViewProps) {
  const merged = mergeProps(
    { variant: 'direct' as const, groupWindow: 5, dayDividers: true, showTimes: true, markdown: true },
    props,
  );
  const byId = createMemo(() => new Map(merged.participants.map((p) => [p.id, p])));

  const entries = createMemo<Entry[]>(() => {
    const out: Entry[] = [];
    let group: Extract<Entry, { k: 'group' }> | null = null;
    let lastDay: string | null = null;
    let lastAt: ChatMessageData['at'] | undefined;
    const close = () => {
      if (group) out.push(group);
      group = null;
    };
    for (const item of merged.items) {
      const at = item.type === 'divider' ? undefined : item.at;
      if (merged.dayDividers && at !== undefined) {
        const day = dayKey(at);
        if (day !== lastDay) {
          close();
          out.push({ k: 'day', id: `fchat-day-${day}`, label: formatDay(at) });
          lastDay = day;
        }
      }
      if (item.type === 'event') {
        close();
        out.push({ k: 'event', id: item.id, item });
      } else if (item.type === 'divider') {
        close();
        out.push({ k: 'divider', id: item.id, item });
      } else {
        const gapOk =
          lastAt === undefined ||
          item.at === undefined ||
          +new Date(item.at) - +new Date(lastAt) < merged.groupWindow * 60_000;
        if (!group || group.author !== item.author || !gapOk) {
          close();
          group = { k: 'group', id: item.id, author: item.author, at: item.at, messages: [] };
        }
        group.messages.push(item);
        lastAt = item.at;
      }
      if (merged.unreadAfter !== undefined && item.id === merged.unreadAfter) {
        close();
        out.push({ k: 'unread', id: `fchat-unread` });
      }
    }
    close();
    return out;
  });

  const typers = createMemo(() =>
    (merged.typing ?? []).map((id) => byId().get(id)?.name ?? id).filter(Boolean),
  );

  /* ------ scroll behavior ------ */
  let scroller!: HTMLDivElement;
  let list!: HTMLDivElement;
  const [pinned, setPinned] = createSignal(true);
  const [newCount, setNewCount] = createSignal(0);
  let topLatched = false;
  let prevFirst: string | undefined;
  let prevLast: string | undefined;
  let prevHeight = 0;

  const toBottom = () => {
    scroller.scrollTop = scroller.scrollHeight;
  };
  const jump = () => {
    toBottom();
    setPinned(true);
    setNewCount(0);
  };
  const onScroll = () => {
    const nb = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < NEAR_BOTTOM;
    setPinned(nb);
    if (nb) setNewCount(0);
    if (scroller.scrollTop < NEAR_TOP) {
      if (!topLatched) {
        topLatched = true;
        merged.onReachTop?.();
      }
    } else {
      topLatched = false;
    }
  };

  onMount(() => {
    toBottom();
    prevHeight = scroller.scrollHeight;
    /* Re-stick as async content (images, link cards) grows the list. */
    const ro = new ResizeObserver(() => {
      if (pinned()) toBottom();
    });
    ro.observe(list);
    onCleanup(() => ro.disconnect());
  });

  createEffect(() => {
    const items = merged.items;
    const first = items[0]?.id;
    const last = items[items.length - 1]?.id;
    if (prevLast !== undefined && last !== prevLast) {
      if (pinned()) {
        toBottom();
      } else {
        const idx = items.findIndex((i) => i.id === prevLast);
        setNewCount((c) => c + (idx >= 0 ? items.length - 1 - idx : 1));
      }
    }
    if (prevFirst !== undefined && first !== prevFirst && last === prevLast && !pinned()) {
      /* History prepended — keep the viewport anchored to the old content. */
      scroller.scrollTop += scroller.scrollHeight - prevHeight;
      topLatched = false;
    }
    prevFirst = first;
    prevLast = last;
    prevHeight = scroller.scrollHeight;
  });

  const showAvatar = (author: string) => merged.variant === 'room' || author !== merged.self;
  const showName = () => merged.variant === 'room';

  return (
    <div class={`fchat ${merged.class ?? ''}`} style={merged.style}>
      <div class="fchat-scrollwrap">
        <div class="fchat-scroll" ref={scroller} role="log" aria-label="Conversation" onScroll={onScroll}>
          <div class="fchat-list" ref={list}>
            <For each={entries()}>
              {(entry) => (
                <Switch>
                  <Match when={entry.k === 'day' && entry}>
                    {(e) => <ChatDivider label={(e() as Extract<Entry, { k: 'day' }>).label} />}
                  </Match>
                  <Match when={entry.k === 'unread'}>
                    <div class="fchat-divider is-unread"><span>New</span></div>
                  </Match>
                  <Match when={entry.k === 'divider' && entry}>
                    {(e) => <ChatDivider label={(e() as Extract<Entry, { k: 'divider' }>).item.label} />}
                  </Match>
                  <Match when={entry.k === 'event' && entry}>
                    {(e) => {
                      const ev = (e() as Extract<Entry, { k: 'event' }>).item;
                      return (
                        <div class="fchat-event">
                          {ev.text}
                          <Show when={ev.at}>
                            <time datetime={isoTime(ev.at!)}>{formatTime(ev.at!)}</time>
                          </Show>
                        </div>
                      );
                    }}
                  </Match>
                  <Match when={entry.k === 'group' && entry}>
                    {(e) => {
                      const g = () => e() as Extract<Entry, { k: 'group' }>;
                      const self = () => g().author === merged.self;
                      const who = () => byId().get(g().author);
                      return (
                        <div class="fchat-group" classList={{ 'is-self': self() }}>
                          <div class="fchat-group-gutter">
                            <Show when={showAvatar(g().author)}>
                              <Avatar size="sm" name={who()?.name ?? g().author} src={who()?.avatar}
                                      status={who()?.status} />
                            </Show>
                          </div>
                          <div class="fchat-group-body">
                            <Show when={showName() || (merged.showTimes && g().at)}>
                              <div class="fchat-meta">
                                <Show when={showName()}>
                                  <span class="fchat-meta-name">{who()?.name ?? g().author}</span>
                                </Show>
                                <Show when={merged.showTimes && g().at}>
                                  <time datetime={isoTime(g().at!)}>{formatTime(g().at!)}</time>
                                </Show>
                              </div>
                            </Show>
                            <For each={g().messages}>
                              {(m) => (
                                <ChatMessage
                                  message={m}
                                  participant={who()}
                                  self={self()}
                                  showTime={merged.showTimes}
                                  markdown={merged.markdown}
                                  resolveLink={merged.resolveLink}
                                />
                              )}
                            </For>
                          </div>
                        </div>
                      );
                    }}
                  </Match>
                </Switch>
              )}
            </For>
            <Show when={typers().length}>
              <ChatTyping names={typers()} />
            </Show>
          </div>
        </div>
        <Show when={!pinned() && newCount() > 0}>
          <button type="button" class="fchat-jump" onClick={jump}>
            <ArrowDownSvg />
            {newCount()} new message{newCount() === 1 ? '' : 's'}
          </button>
        </Show>
      </div>
      {merged.children}
    </div>
  );
}

/* ---------------- ChatDivider -------------------------------------------------- */
export function ChatDivider(props: { label: JSX.Element }) {
  return (
    <div class="fchat-divider">
      <span>{props.label}</span>
    </div>
  );
}

/* ---------------- ChatTyping ---------------------------------------------------- */
export function ChatTyping(props: { names: string[] }) {
  const text = () => {
    const n = props.names;
    if (n.length === 1) return `${n[0]} is typing`;
    if (n.length === 2) return `${n[0]} and ${n[1]} are typing`;
    return 'Several people are typing';
  };
  return (
    <div class="fchat-typing" aria-live="polite">
      <span class="fchat-typing-dots" aria-hidden="true">
        <span /><span /><span />
      </span>
      {text()}
    </div>
  );
}
