import { For, Show, createSignal, mergeProps, splitProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { ChevronDown } from './internal/icons';

/* ---------------- Table ---------------------------------------------------- */
/* Markup-only: pass thead/tbody as children. The wrap div gives wide tables
   horizontal scroll at <=768px. */
export function Table(props: JSX.HTMLAttributes<HTMLTableElement>) {
  const [local, rest] = splitProps(props, ['children']);
  return (
    <div class="ftable-wrap">
      <table class="ftable" {...rest}>{local.children}</table>
    </div>
  );
}

/* ---------------- Logs ----------------------------------------------------- */
export function Logs(props: JSX.HTMLAttributes<HTMLDivElement>) {
  const [local, rest] = splitProps(props, ['children']);
  return <div class="flogs" {...rest}>{local.children}</div>;
}

export interface LogLineProps {
  time?: JSX.Element;
  level?: 'info' | 'warn' | 'error' | 'debug' | (string & {});
  children?: JSX.Element;
}

export function LogLine(props: LogLineProps) {
  const merged = mergeProps({ level: 'info' }, props);
  return (
    <div class="flog-line">
      <span class="flog-time">{merged.time}</span>
      <span class={`flog-level ${merged.level}`}>{merged.level}</span>
      <span class="flog-msg">{merged.children}</span>
    </div>
  );
}

/* ---------------- Collapsible & Accordion ----------------------------------- */
export interface CollapsibleProps {
  title: JSX.Element;
  defaultOpen?: boolean;
  onToggle?: (open: boolean) => void;
  children?: JSX.Element;
}

export function Collapsible(props: CollapsibleProps) {
  const [open, setOpen] = createSignal(!!props.defaultOpen);
  const toggle = () => { setOpen((o) => !o); props.onToggle?.(open()); };
  return (
    <div class="facc">
      <div class="facc-item" classList={{ 'is-open': open() }}>
        <button type="button" class="facc-head" aria-expanded={open()} onClick={toggle}>
          {props.title}
          <ChevronDown />
        </button>
        <Show when={open()}>
          <div class="facc-body">{props.children}</div>
        </Show>
      </div>
    </div>
  );
}

export interface AccordionItem {
  id: string;
  title: JSX.Element;
  content: JSX.Element;
}

export interface AccordionProps {
  items: AccordionItem[];
  defaultOpen?: string;
}

export function Accordion(props: AccordionProps) {
  const [openId, setOpenId] = createSignal<string | null>(props.defaultOpen ?? null);
  return (
    <div class="facc">
      <For each={props.items}>
        {(item) => (
          <div class="facc-item" classList={{ 'is-open': openId() === item.id }}>
            <button type="button" class="facc-head" aria-expanded={openId() === item.id}
                    onClick={() => setOpenId((cur) => (cur === item.id ? null : item.id))}>
              {item.title}
              <ChevronDown />
            </button>
            <Show when={openId() === item.id}>
              <div class="facc-body">{item.content}</div>
            </Show>
          </div>
        )}
      </For>
    </div>
  );
}
