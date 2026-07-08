/* Forge UI primitives — SolidJS port of ui_kits/console/ui.jsx.
   Requires console.css + colors_and_type.css to be imported by the app.
   Icons come from lucide-solid: pass the imported icon component via the
   `icon` prop, e.g. icon={Terminal}. All icons render at 1.5px stroke. */

import { Show, Index, createSignal, createEffect, onCleanup, mergeProps, splitProps } from 'solid-js';
import { Dynamic, Portal } from 'solid-js/web';

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

/* ===========================================================================
   Shell, page, data and settings components — thin wrappers over the
   console.css classes. Raw-class markup remains fully supported; these are
   additive sugar. Private SVGs below keep ui.jsx usable without lucide-solid.
   =========================================================================== */

const MenuSvg = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
    <line x1="4" y1="6" x2="20" y2="6" /><line x1="4" y1="12" x2="20" y2="12" /><line x1="4" y1="18" x2="20" y2="18" />
  </svg>
);
const XSvg = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
    <line x1="6" y1="6" x2="18" y2="18" /><line x1="18" y1="6" x2="6" y2="18" />
  </svg>
);

/* ---------------- App shell ------------------------------------------------ */
/* Owns the mobile drawer state: renders the hamburger toggle, the scrim, and
   closes the drawer when the sidebar is tapped. Inert above 1024px. */
export function AppShell(props) {
  const [navOpen, setNavOpen] = createSignal(false);
  const close = () => setNavOpen(false);
  return (
    <div class="app-shell" classList={{ 'is-sidebar-open': navOpen() }}>
      <header class="ftopbar">
        <button class="ftopbar-icon-btn fsidebar-toggle" aria-label="Toggle navigation"
                aria-expanded={navOpen()} onClick={() => setNavOpen((o) => !o)}>
          <MenuSvg />
        </button>
        {props.topbar}
      </header>
      <nav class="fsidebar" onClick={close}>{props.sidebar}</nav>
      <div class="fscrim" onClick={close} />
      <main class="app-main">{props.children}</main>
    </div>
  );
}

export function NavSection(props) {
  return <div class="fsidebar-section">{props.children}</div>;
}

export function NavLink(props) {
  const [local, rest] = splitProps(props, ['icon', 'active', 'count', 'children']);
  return (
    <a classList={{ 'is-active': !!local.active }} {...rest}>
      <Show when={local.icon}>
        <Icon of={local.icon} size={14} />
      </Show>
      {local.children}
      <Show when={local.count != null}>
        <span class="count">{local.count}</span>
      </Show>
    </a>
  );
}

export function Crumbs(props) {
  return (
    <div class="ftopbar-crumbs">
      <Index each={props.items}>
        {(item, i) => (
          <>
            <Show when={i > 0}>
              <span class="sep">/</span>
            </Show>
            <span>{item()}</span>
          </>
        )}
      </Index>
    </div>
  );
}

export function IconButton(props) {
  const [local, rest] = splitProps(props, ['icon', 'label']);
  return (
    <button class="ftopbar-icon-btn" aria-label={local.label} title={local.label} {...rest}>
      <Icon of={local.icon} />
    </button>
  );
}

/* ---------------- Page ----------------------------------------------------- */
export function PageHead(props) {
  return (
    <div class="page-head">
      <div>
        <h1 style={{ 'font-size': '22px' }}>{props.title}</h1>
        <Show when={props.sub}>
          <div class="sub">{props.sub}</div>
        </Show>
      </div>
      <Show when={props.actions}>
        <div class="page-actions">{props.actions}</div>
      </Show>
    </div>
  );
}

export function Grid(props) {
  const [local, rest] = splitProps(props, ['children']);
  return <div class="fgrid" {...rest}>{local.children}</div>;
}

export function Empty(props) {
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

export function Eyebrow(props) {
  return <div class="eyebrow">{props.children}</div>;
}

/* ---------------- Table ---------------------------------------------------- */
/* Markup-only: pass thead/tbody as children (see the dashboard-card example).
   The wrap div gives wide tables horizontal scroll at <=768px. */
export function Table(props) {
  const [local, rest] = splitProps(props, ['children']);
  return (
    <div class="ftable-wrap">
      <table class="ftable" {...rest}>{local.children}</table>
    </div>
  );
}

/* ---------------- Logs ----------------------------------------------------- */
export function Logs(props) {
  const [local, rest] = splitProps(props, ['children']);
  return <div class="flogs" {...rest}>{local.children}</div>;
}

export function LogLine(props) {
  const merged = mergeProps({ level: 'info' }, props);
  return (
    <div class="flog-line">
      <span class="flog-time">{merged.time}</span>
      <span class={`flog-level ${merged.level}`}>{merged.level}</span>
      <span class="flog-msg">{merged.children}</span>
    </div>
  );
}

/* ---------------- Settings ------------------------------------------------- */
export function SettingsLayout(props) {
  return (
    <div class="settings-layout">
      <nav class="settings-nav">{props.nav}</nav>
      <div>{props.children}</div>
    </div>
  );
}

export function SettingsSection(props) {
  return (
    <section class="settings-section">
      <h2>{props.title}</h2>
      <Show when={props.sub}>
        <p class="sub">{props.sub}</p>
      </Show>
      {props.children}
    </section>
  );
}

export function SettingsRow(props) {
  return <div class="settings-row">{props.children}</div>;
}

/* ---------------- Modal ---------------------------------------------------- */
/* Controlled: <Modal open={sig()} onClose={...} title footer>body</Modal>.
   Closes on Escape, backdrop click, and the head X. */
export function Modal(props) {
  createEffect(() => {
    if (!props.open) return;
    const onKey = (e) => { if (e.key === 'Escape') props.onClose?.(); };
    document.addEventListener('keydown', onKey);
    onCleanup(() => document.removeEventListener('keydown', onKey));
  });
  return (
    <Show when={props.open}>
      <Portal>
        <div class="fmodal" onClick={(e) => { if (e.target === e.currentTarget) props.onClose?.(); }}>
          <div class="fmodal-panel" role="dialog" aria-modal="true" aria-label={props.title}>
            <header class="fmodal-head">
              <h3>{props.title}</h3>
              <button class="ftopbar-icon-btn" aria-label="Close" onClick={() => props.onClose?.()}>
                <XSvg />
              </button>
            </header>
            <div class="fmodal-body">{props.children}</div>
            <Show when={props.footer}>
              <footer class="fmodal-foot">{props.footer}</footer>
            </Show>
          </div>
        </div>
      </Portal>
    </Show>
  );
}
