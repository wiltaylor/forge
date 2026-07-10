import { Show, createEffect, createSignal, mergeProps, onMount } from 'solid-js';
import type { JSX } from 'solid-js';
import { SendSvg } from './internal/icons';

/* Auto-growing message input. Enter sends (IME-safe), Shift+Enter breaks. */
export interface ChatComposerProps {
  /** Controlled value; leave undefined for internal state. */
  value?: string;
  onChange?: (value: string) => void;
  /** Called with the trimmed text — never fired empty. */
  onSend: (text: string) => void;
  placeholder?: string;
  disabled?: boolean;
  /** aria-label of the send button (default "Send"). */
  sendLabel?: string;
  /** Left slot for attachment/action buttons. */
  actions?: JSX.Element;
  /** Row above the field, e.g. attachment chips. */
  accessories?: JSX.Element;
  /** Textarea grows up to this many rows, then scrolls (default 8). */
  maxRows?: number;
  autofocus?: boolean;
}

export function ChatComposer(props: ChatComposerProps) {
  const merged = mergeProps({ placeholder: 'Message', sendLabel: 'Send', maxRows: 8 }, props);
  const [inner, setInner] = createSignal('');
  const value = () => merged.value ?? inner();
  let area!: HTMLTextAreaElement;

  const setValue = (v: string) => {
    setInner(v);
    merged.onChange?.(v);
  };
  const autogrow = () => {
    area.style.height = 'auto';
    const line = parseFloat(getComputedStyle(area).lineHeight) || 20;
    const max = line * merged.maxRows;
    area.style.height = `${Math.min(area.scrollHeight, max)}px`;
    area.style.overflowY = area.scrollHeight > max ? 'auto' : 'hidden';
  };
  const send = () => {
    const text = value().trim();
    if (!text || merged.disabled) return;
    merged.onSend(text);
    setValue('');
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
    }
  };

  onMount(() => {
    if (merged.autofocus) area.focus();
  });
  createEffect(() => {
    value();
    autogrow();
  });

  return (
    <div class="fchat-composer">
      <Show when={merged.accessories}>
        <div class="fchat-composer-accessories">{merged.accessories}</div>
      </Show>
      <div class="fchat-composer-row">
        <Show when={merged.actions}>
          <div class="fchat-composer-actions">{merged.actions}</div>
        </Show>
        <div class="fchat-composer-field">
          <textarea
            ref={area}
            rows="1"
            value={value()}
            placeholder={merged.placeholder}
            disabled={merged.disabled}
            onInput={(e) => setValue(e.currentTarget.value)}
            onKeyDown={onKeyDown}
          />
        </div>
        <button
          type="button"
          class="fchat-composer-send"
          aria-label={merged.sendLabel}
          title={merged.sendLabel}
          disabled={merged.disabled || !value().trim()}
          onClick={send}
        >
          <SendSvg />
        </button>
      </div>
    </div>
  );
}
