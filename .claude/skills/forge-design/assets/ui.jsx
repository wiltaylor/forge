/* Forge UI primitives — SolidJS port of ui_kits/console/ui.jsx.
   Requires console.css + colors_and_type.css to be imported by the app.
   Icons come from lucide-solid: pass the imported icon component via the
   `icon` prop, e.g. icon={Terminal}. All icons render at 1.5px stroke. */

import { Show, mergeProps, splitProps } from 'solid-js';
import { Dynamic } from 'solid-js/web';

/* ---------------- Icon ----------------------------------------------------- */
/* Wraps a lucide-solid icon component with Forge defaults (16px, 1.5 stroke). */
export function Icon(props) {
  const merged = mergeProps({ size: 16 }, props);
  const [local, rest] = splitProps(merged, ['of', 'size']);
  return <Dynamic component={local.of} size={local.size} strokeWidth={1.5} aria-hidden="true" {...rest} />;
}

/* ---------------- Button --------------------------------------------------- */
export function Button(props) {
  const merged = mergeProps({ variant: 'secondary', size: 'md' }, props);
  const [local, rest] = splitProps(merged, ['variant', 'size', 'icon', 'children', 'class']);
  return (
    <button
      class={`fbtn fbtn-${local.variant} fbtn-${local.size} ${local.class ?? ''}`}
      {...rest}
    >
      <Show when={local.icon}>
        <Icon of={local.icon} size={local.size === 'sm' ? 12 : 14} />
      </Show>
      {local.children}
    </button>
  );
}

/* ---------------- Input ---------------------------------------------------- */
export function Input(props) {
  const [local, rest] = splitProps(props, ['icon', 'error', 'label', 'help']);
  return (
    <label class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <span class="ffield-input" classList={{ 'is-error': !!local.error }}>
        <Show when={local.icon}>
          <Icon of={local.icon} size={14} />
        </Show>
        <input {...rest} />
      </span>
      <Show when={local.help}>
        <span class="ffield-help" classList={{ 'is-error': !!local.error }}>{local.help}</span>
      </Show>
    </label>
  );
}

/* ---------------- Badge ---------------------------------------------------- */
export function Badge(props) {
  const merged = mergeProps({ tone: 'neutral' }, props);
  return (
    <span class={`fbadge fbadge-${merged.tone}`}>
      <Show when={merged.dot}>
        <span class="fbadge-dot" />
      </Show>
      {merged.children}
    </span>
  );
}

/* ---------------- Card ----------------------------------------------------- */
export function Card(props) {
  const merged = mergeProps({ padded: true }, props);
  return (
    <section class={`fcard ${merged.class ?? ''}`}>
      <Show when={merged.title}>
        <header class="fcard-head">
          <h3 class="fcard-title">{merged.title}</h3>
          {merged.action}
        </header>
      </Show>
      <div class={merged.padded ? 'fcard-body' : ''}>{merged.children}</div>
    </section>
  );
}

/* ---------------- Stat ----------------------------------------------------- */
export function Stat(props) {
  return (
    <div class="fstat">
      <div class="eyebrow">{props.label}</div>
      <div class="fstat-value">{props.value}</div>
      <Show when={props.delta}>
        <div class={`fstat-delta tone-${props.tone ?? 'neutral'}`}>{props.delta}</div>
      </Show>
    </div>
  );
}

/* ---------------- Kbd ------------------------------------------------------ */
export function Kbd(props) {
  return <kbd class="fkbd">{props.children}</kbd>;
}

/* ---------------- Toast ---------------------------------------------------- */
export function Toast(props) {
  const merged = mergeProps({ tone: 'info' }, props);
  return (
    <div class={`ftoast ftoast-${merged.tone}`}>
      <Show when={merged.icon}>
        <Icon of={merged.icon} size={14} />
      </Show>
      <span>{merged.children}</span>
    </div>
  );
}

/* ---------------- Status dot ---------------------------------------------- */
export function StatusDot(props) {
  return <span class={`fdot fdot-${props.tone}`} />;
}
