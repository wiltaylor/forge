import { Index, Show, createSignal, mergeProps, splitProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { MenuSvg } from './internal/icons';
import { Icon } from './primitives';
import { ChevronLeftSvg, ChevronRightSvg } from './internal/icons';
import { For } from 'solid-js';
import type { IconComponent } from './types';

/* ---------------- App shell ------------------------------------------------ */
/* Owns the mobile drawer state: renders the hamburger toggle, the scrim, and
   closes the drawer when the sidebar is tapped. Inert above 1024px. */
export interface AppShellProps {
  topbar?: JSX.Element;
  sidebar?: JSX.Element;
  children?: JSX.Element;
}

export function AppShell(props: AppShellProps) {
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

export function NavSection(props: { children?: JSX.Element }) {
  return <div class="fsidebar-section">{props.children}</div>;
}

export interface NavLinkProps extends JSX.AnchorHTMLAttributes<HTMLAnchorElement> {
  icon?: IconComponent;
  active?: boolean;
  count?: number | string;
}

export function NavLink(props: NavLinkProps) {
  const [local, rest] = splitProps(props, ['icon', 'active', 'count', 'children']);
  return (
    <a classList={{ 'is-active': !!local.active }} {...rest}>
      <Show when={local.icon}>
        <Icon of={local.icon!} size={14} />
      </Show>
      {local.children}
      <Show when={local.count != null}>
        <span class="count">{local.count}</span>
      </Show>
    </a>
  );
}

export function Crumbs(props: { items: JSX.Element[] }) {
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

/* ---------------- Page ----------------------------------------------------- */
export interface PageHeadProps {
  title: JSX.Element;
  sub?: JSX.Element;
  actions?: JSX.Element;
}

export function PageHead(props: PageHeadProps) {
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

/* ---------------- Tabs (bar only — content is the consumer's Show/Switch) --- */
export interface TabItem {
  id: string;
  label: JSX.Element;
  count?: number | string;
  disabled?: boolean;
}

export interface TabsProps {
  tabs: TabItem[];
  active?: string;
  onChange?: (id: string) => void;
}

export function Tabs(props: TabsProps) {
  return (
    <div class="ftabs" role="tablist">
      <For each={props.tabs}>
        {(t) => (
          <button type="button" class="ftab" role="tab" disabled={t.disabled}
                  aria-selected={props.active === t.id}
                  classList={{ 'is-active': props.active === t.id }}
                  onClick={() => props.onChange?.(t.id)}>
            {t.label}
            <Show when={t.count != null}>
              <span class="count">{t.count}</span>
            </Show>
          </button>
        )}
      </For>
    </div>
  );
}

/* ---------------- Pagination ------------------------------------------------ */
export interface PaginationProps {
  page: number;
  pages: number;
  onChange?: (page: number) => void;
}

export function Pagination(props: PaginationProps) {
  const window_ = (): (number | '…')[] => {
    const { page, pages } = props;
    if (pages <= 7) return Array.from({ length: pages }, (_, i) => i + 1);
    const items: (number | '…')[] = [1];
    if (page > 3) items.push('…');
    for (let p = Math.max(2, page - 1); p <= Math.min(pages - 1, page + 1); p++) items.push(p);
    if (page < pages - 2) items.push('…');
    items.push(pages);
    return items;
  };
  return (
    <nav class="fpage" aria-label="Pagination">
      <button type="button" class="fpage-btn" aria-label="Previous page"
              disabled={props.page <= 1} onClick={() => props.onChange?.(props.page - 1)}>
        <ChevronLeftSvg />
      </button>
      <For each={window_()}>
        {(p) => (
          <Show when={p !== '…'} fallback={<span class="fpage-gap">…</span>}>
            <button type="button" class="fpage-btn"
                    classList={{ 'is-active': p === props.page }}
                    aria-current={p === props.page ? 'page' : undefined}
                    onClick={() => props.onChange?.(p as number)}>
              {p}
            </button>
          </Show>
        )}
      </For>
      <button type="button" class="fpage-btn" aria-label="Next page"
              disabled={props.page >= props.pages} onClick={() => props.onChange?.(props.page + 1)}>
        <ChevronRightSvg />
      </button>
    </nav>
  );
}

/* ---------------- SplitPane ------------------------------------------------- */
export interface SplitPaneProps {
  first: JSX.Element;
  second: JSX.Element;
  initial?: number;
  min?: number;
  vertical?: boolean;
  onResize?: (px: number) => void;
  class?: string;
  style?: JSX.CSSProperties;
}

export function SplitPane(props: SplitPaneProps) {
  const merged = mergeProps({ initial: 280, min: 160 }, props);
  const [size, setSize] = createSignal(merged.initial);
  const [dragging, setDragging] = createSignal(false);
  let root!: HTMLDivElement;

  const clamp = (px: number) => {
    const total = merged.vertical ? root.clientHeight : root.clientWidth;
    return Math.min(Math.max(px, merged.min), Math.max(merged.min, total - merged.min - 4));
  };
  const apply = (px: number) => {
    const v = clamp(px);
    setSize(v);
    merged.onResize?.(v);
  };
  const dividerDown = (e: PointerEvent & { currentTarget: HTMLDivElement }) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    setDragging(true);
  };
  const dividerMove = (e: PointerEvent) => {
    if (!dragging()) return;
    const r = root.getBoundingClientRect();
    apply(merged.vertical ? e.clientY - r.top : e.clientX - r.left);
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') { e.preventDefault(); apply(size() - 16); }
    else if (e.key === 'ArrowRight' || e.key === 'ArrowDown') { e.preventDefault(); apply(size() + 16); }
  };

  return (
    <div class={`fsplit ${merged.class ?? ''}`} classList={{ 'is-vertical': !!merged.vertical }}
         style={merged.style} ref={root}>
      <div class="fsplit-pane" style={merged.vertical ? { height: `${size()}px`, flex: 'none' } : { width: `${size()}px`, flex: 'none' }}>
        {merged.first}
      </div>
      <div class="fsplit-divider" role="separator" tabindex="0"
           aria-orientation={merged.vertical ? 'horizontal' : 'vertical'}
           classList={{ 'is-dragging': dragging() }}
           onPointerDown={dividerDown} onPointerMove={dividerMove}
           onPointerUp={() => setDragging(false)} onKeyDown={onKeyDown} />
      <div class="fsplit-pane" style={{ flex: 1 }}>{merged.second}</div>
    </div>
  );
}

/* ---------------- Settings ------------------------------------------------- */
export function SettingsLayout(props: { nav?: JSX.Element; children?: JSX.Element }) {
  return (
    <div class="settings-layout">
      <nav class="settings-nav">{props.nav}</nav>
      <div>{props.children}</div>
    </div>
  );
}

export function SettingsSection(props: { title: JSX.Element; sub?: JSX.Element; children?: JSX.Element }) {
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

export function SettingsRow(props: { children?: JSX.Element }) {
  return <div class="settings-row">{props.children}</div>;
}
