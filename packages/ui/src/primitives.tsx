import { Show, mergeProps, splitProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import type { ControlSize, IconComponent, StatusTone, Tone } from './types';

/* ---------------- Icon ----------------------------------------------------- */
/* Wraps a consumer-provided icon component (e.g. lucide-solid) with Forge
   defaults (16px, 1.5 stroke). */
export interface IconProps {
  of: IconComponent;
  size?: number | string;
  [prop: string]: unknown;
}

export function Icon(props: IconProps) {
  const merged = mergeProps({ size: 16 }, props);
  const [local, rest] = splitProps(merged, ['of', 'size']);
  return <Dynamic component={local.of} size={local.size} strokeWidth={1.5} aria-hidden="true" {...rest} />;
}

/* ---------------- Button --------------------------------------------------- */
export interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: ControlSize;
  icon?: IconComponent;
}

export function Button(props: ButtonProps) {
  const merged = mergeProps({ variant: 'secondary' as const, size: 'md' as const }, props);
  const [local, rest] = splitProps(merged, ['variant', 'size', 'icon', 'children', 'class']);
  return (
    <button
      class={`fbtn fbtn-${local.variant} fbtn-${local.size} ${local.class ?? ''}`}
      {...rest}
    >
      <Show when={local.icon}>
        <Icon of={local.icon!} size={local.size === 'sm' ? 12 : 14} />
      </Show>
      {local.children}
    </button>
  );
}

/* ---------------- IconButton ----------------------------------------------- */
export interface IconButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: IconComponent;
  label: string;
}

export function IconButton(props: IconButtonProps) {
  const [local, rest] = splitProps(props, ['icon', 'label']);
  return (
    <button class="ftopbar-icon-btn" aria-label={local.label} title={local.label} {...rest}>
      <Icon of={local.icon} />
    </button>
  );
}

/* ---------------- Badge ---------------------------------------------------- */
export interface BadgeProps {
  tone?: Tone;
  dot?: boolean;
  children?: JSX.Element;
}

export function Badge(props: BadgeProps) {
  const merged = mergeProps({ tone: 'neutral' as const }, props);
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
export interface CardProps {
  title?: JSX.Element;
  action?: JSX.Element;
  padded?: boolean;
  class?: string;
  children?: JSX.Element;
}

export function Card(props: CardProps) {
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
export interface StatProps {
  label: JSX.Element;
  value: JSX.Element;
  delta?: JSX.Element;
  tone?: Tone;
}

export function Stat(props: StatProps) {
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
export function Kbd(props: { children?: JSX.Element }) {
  return <kbd class="fkbd">{props.children}</kbd>;
}

/* ---------------- Status dot ---------------------------------------------- */
export function StatusDot(props: { tone: StatusTone }) {
  return <span class={`fdot fdot-${props.tone}`} />;
}

/* ---------------- Separator ------------------------------------------------- */
export function Separator(props: { vertical?: boolean }) {
  return <div class="fsep" classList={{ 'is-vertical': !!props.vertical }} role="separator" />;
}

/* ---------------- Skeleton -------------------------------------------------- */
export interface SkeletonProps extends Omit<JSX.HTMLAttributes<HTMLDivElement>, 'style'> {
  width?: string;
  height?: string;
  style?: JSX.CSSProperties;
}

export function Skeleton(props: SkeletonProps) {
  const [local, rest] = splitProps(props, ['width', 'height', 'style']);
  return (
    <div class="fskel" aria-hidden="true"
         style={{ width: local.width, height: local.height, ...local.style }} {...rest} />
  );
}

/* ---------------- Avatar ---------------------------------------------------- */
export interface AvatarProps {
  name?: string;
  src?: string;
  size?: ControlSize;
  status?: StatusTone;
}

export function Avatar(props: AvatarProps) {
  const merged = mergeProps({ size: 'md' as const }, props);
  const initials = () => (merged.name ?? '')
    .split(/\s+/).slice(0, 2).map((w) => w[0] ?? '').join('').toUpperCase();
  return (
    <span class={`favatar favatar-${merged.size}`} title={merged.name}>
      <Show when={merged.src} fallback={initials()}>
        <img src={merged.src} alt={merged.name} />
      </Show>
      <Show when={merged.status}>
        <span class={`favatar-status fdot-${merged.status}`}
              style={{ background: `var(--${merged.status === 'neutral' ? 'fg-3' : merged.status})` }} />
      </Show>
    </span>
  );
}

/* ---------------- Eyebrow / Empty / Grid ------------------------------------ */
export function Eyebrow(props: { children?: JSX.Element }) {
  return <div class="eyebrow">{props.children}</div>;
}

export interface EmptyProps {
  title: JSX.Element;
  action?: JSX.Element;
  children?: JSX.Element;
}

export function Empty(props: EmptyProps) {
  return (
    <div class="empty">
      <h3>{props.title}</h3>
      <Show when={props.children}>
        <p>{props.children}</p>
      </Show>
      {props.action}
    </div>
  );
}

export function Grid(props: JSX.HTMLAttributes<HTMLDivElement>) {
  const [local, rest] = splitProps(props, ['children']);
  return <div class="fgrid" {...rest}>{local.children}</div>;
}
