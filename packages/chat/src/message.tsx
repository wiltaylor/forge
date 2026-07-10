import { For, Match, Show, Switch } from 'solid-js';
import type { JSX } from 'solid-js';
import { Icon } from '@forge/ui';
import type { ChatBlock, ChatMessageData, ChatParticipant, LinkResolver } from './types';
import { Markdown } from './markdown';
import { ChatToolCall } from './toolcall';
import { ChatPrompt } from './prompt';
import { LinkCard } from './linkcard';
import { FileSvg } from './internal/icons';
import { formatBytes, formatTime, isoTime } from './internal/time';

/* One message: text blocks render as bubbles, everything else (media, files,
   link cards, tool calls, prompts) as standalone rows in the message column. */
export interface ChatMessageProps {
  message: ChatMessageData;
  participant?: ChatParticipant;
  self?: boolean;
  /** First message of its group — ChatView shows avatar/name/time above it. */
  groupStart?: boolean;
  showTime?: boolean;
  markdown?: boolean;
  resolveLink?: LinkResolver;
}

export function ChatMessage(props: ChatMessageProps) {
  const blocks = (): ChatBlock[] =>
    props.message.blocks ??
    (props.message.text !== undefined ? [{ kind: 'text', text: props.message.text }] : []);
  const label = () => {
    const who = props.participant?.name ?? props.message.author;
    return props.message.at ? `${who}, ${formatTime(props.message.at)}` : who;
  };

  return (
    <article
      class="fchat-msg"
      classList={{ 'is-pending': !!props.message.pending, 'is-error': !!props.message.error }}
      aria-label={label()}
    >
      <div class="fchat-msg-blocks">
        <For each={blocks()}>{(block) => <Block block={block} {...props} />}</For>
        <Show when={props.message.error}>
          <div class="fchat-msg-fail">{props.message.error}</div>
        </Show>
      </div>
      <Show when={props.showTime !== false && props.message.at}>
        <time class="fchat-msg-time" datetime={isoTime(props.message.at!)}>
          {formatTime(props.message.at!)}
        </time>
      </Show>
    </article>
  );
}

function Block(props: ChatMessageProps & { block: ChatBlock }) {
  const b = () => props.block;
  return (
    <Switch>
      <Match when={b().kind === 'text' && b()}>
        {(block) => {
          const t = block() as Extract<ChatBlock, { kind: 'text' }>;
          return (
            <div class="fchat-bubble">
              <Show when={(t.markdown ?? props.markdown) !== false} fallback={<p class="fchat-plain">{t.text}</p>}>
                <Markdown text={t.text} />
              </Show>
            </div>
          );
        }}
      </Match>
      <Match when={b().kind === 'image' && b()}>
        {(block) => {
          const img = block() as Extract<ChatBlock, { kind: 'image' }>;
          const media = (
            <span class="fchat-media" style={mediaStyle(img.width, img.height)}>
              <img src={img.src} alt={img.alt ?? ''} loading="lazy" />
            </span>
          );
          return (
            <Show when={img.href} fallback={media}>
              <a href={img.href} target="_blank" rel="noopener noreferrer">{media}</a>
            </Show>
          );
        }}
      </Match>
      <Match when={b().kind === 'video' && b()}>
        {(block) => {
          const v = block() as Extract<ChatBlock, { kind: 'video' }>;
          return (
            <span class="fchat-media is-video" style={mediaStyle(v.width, v.height)}>
              <video src={v.src} poster={v.poster} controls preload="metadata" />
            </span>
          );
        }}
      </Match>
      <Match when={b().kind === 'file' && b()}>
        {(block) => {
          const f = block() as Extract<ChatBlock, { kind: 'file' }>;
          const row = (
            <>
              <Show when={f.icon} fallback={<FileSvg />}>
                <Icon of={f.icon!} size={15} />
              </Show>
              <span class="fchat-file-name">{f.name}</span>
              <Show when={f.size !== undefined}>
                <span class="fchat-file-size">{formatBytes(f.size!)}</span>
              </Show>
            </>
          );
          return (
            <Show when={f.href} fallback={<span class="fchat-file">{row}</span>}>
              <a class="fchat-file" href={f.href} download={f.name}>{row}</a>
            </Show>
          );
        }}
      </Match>
      <Match when={b().kind === 'link' && b()}>
        {(block) => {
          const l = block() as Extract<ChatBlock, { kind: 'link' }>;
          return <LinkCard url={l.url} meta={l.meta} resolve={props.resolveLink} />;
        }}
      </Match>
      <Match when={b().kind === 'tool' && b()}>
        {(block) => <ChatToolCall tool={(block() as Extract<ChatBlock, { kind: 'tool' }>).tool} />}
      </Match>
      <Match when={b().kind === 'prompt' && b()}>
        {(block) => <ChatPrompt prompt={(block() as Extract<ChatBlock, { kind: 'prompt' }>).prompt} />}
      </Match>
      <Match when={b().kind === 'custom' && b()}>
        {(block) => (block() as Extract<ChatBlock, { kind: 'custom' }>).render()}
      </Match>
    </Switch>
  );
}

function mediaStyle(width?: number, height?: number): JSX.CSSProperties {
  const style: JSX.CSSProperties = {};
  if (width && height) style['aspect-ratio'] = `${width} / ${height}`;
  if (width) style.width = `min(${width}px, 100%)`;
  return style;
}
