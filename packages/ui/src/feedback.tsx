import { Show, mergeProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { Icon } from './primitives';
import type { IconComponent, Tone } from './types';

/* ---------------- Toast (presentational; see toaster.tsx for the manager) --- */
export interface ToastProps {
  tone?: Tone;
  icon?: IconComponent;
  children?: JSX.Element;
}

export function Toast(props: ToastProps) {
  const merged = mergeProps({ tone: 'info' as const }, props);
  return (
    <div class={`ftoast ftoast-${merged.tone}`}>
      <Show when={merged.icon}>
        <Icon of={merged.icon!} size={14} />
      </Show>
      <span>{merged.children}</span>
    </div>
  );
}

/* ---------------- Alert ----------------------------------------------------- */
export interface AlertProps {
  tone?: Tone;
  icon?: IconComponent;
  title?: JSX.Element;
  children?: JSX.Element;
}

export function Alert(props: AlertProps) {
  const merged = mergeProps({ tone: 'info' as const }, props);
  return (
    <div class={`falert falert-${merged.tone}`} role="alert">
      <Show when={merged.icon}>
        <Icon of={merged.icon!} size={15} />
      </Show>
      <div>
        <Show when={merged.title}>
          <div class="falert-title">{merged.title}</div>
        </Show>
        <div class="falert-body">{merged.children}</div>
      </div>
    </div>
  );
}

/* ---------------- Progress -------------------------------------------------- */
/* Thin bar — the default Forge loading treatment. */
export interface ProgressProps {
  value?: number;
  tone?: Tone;
  indeterminate?: boolean;
  label?: string;
  showValue?: boolean;
}

export function Progress(props: ProgressProps) {
  const merged = mergeProps({ tone: 'accent' as const, value: 0 }, props);
  return (
    <div class="fprogress" classList={{ 'is-indeterminate': !!merged.indeterminate }}>
      <Show when={merged.label || merged.showValue}>
        <div class="fprogress-head">
          <span>{merged.label}</span>
          <Show when={merged.showValue && !merged.indeterminate}>
            <span class="fprogress-value">{Math.round(merged.value)} %</span>
          </Show>
        </div>
      </Show>
      <div class="fprogress-track" role="progressbar"
           aria-valuemin="0" aria-valuemax="100"
           aria-valuenow={merged.indeterminate ? undefined : Math.round(merged.value)}
           aria-label={merged.label}>
        <div class={`fprogress-fill${merged.tone !== 'accent' ? ` tone-${merged.tone}` : ''}`}
             style={merged.indeterminate ? undefined : { width: `${Math.min(100, Math.max(0, merged.value))}%` }} />
      </div>
    </div>
  );
}

/* ---------------- Spinner --------------------------------------------------- */
/* For inline/button-adjacent waits — thin Progress stays the default (tokens.md). */
export interface SpinnerProps {
  size?: number;
  label?: string;
}

export function Spinner(props: SpinnerProps) {
  const merged = mergeProps({ size: 16, label: 'Loading' }, props);
  return (
    <svg class="fspinner" width={merged.size} height={merged.size} viewBox="0 0 24 24"
         fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"
         role="status" aria-label={merged.label}>
      <circle cx="12" cy="12" r="9" opacity="0.25" />
      <path d="M12 3 a 9 9 0 0 1 9 9" />
    </svg>
  );
}
