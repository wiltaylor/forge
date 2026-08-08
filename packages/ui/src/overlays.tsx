import { For, Show, createEffect, createSignal, mergeProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { SearchSvg, XSvg } from './internal/icons';
import { MenuList } from './internal/menu';
import { OverlayPortal, useOverlay } from './overlay';
import { Button, Icon, Kbd } from './primitives';
import type { CommandItem, ControlSize, IconComponent, MenuItem } from './types';

/* ---------------- Modal ---------------------------------------------------- */
/* Controlled: <Modal open={sig()} onClose={...} title footer>body</Modal>.
   Closes on Escape, backdrop click, and the head X. */
export interface ModalProps {
  open?: boolean;
  onClose?: () => void;
  title?: string;
  /** Panel width: md 480px (default), lg 720px, xl 960px. */
  size?: 'md' | 'lg' | 'xl';
  footer?: JSX.Element;
  children?: JSX.Element;
}

export function Modal(props: ModalProps): JSX.Element {
  let backdrop: HTMLDivElement | undefined;
  let panel: HTMLDivElement | undefined;
  useOverlay({
    open: () => !!props.open,
    surface: () => panel,
    backdrop: () => backdrop,
    onDismiss: () => props.onClose?.(),
  });
  return (
    <Show when={props.open}>
      <OverlayPortal>
        <div ref={backdrop} class="fmodal">
          <div ref={panel} class="fmodal-panel" classList={{ 'is-lg': props.size === 'lg', 'is-xl': props.size === 'xl' }} role="dialog" aria-modal="true" aria-label={props.title}>
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
      </OverlayPortal>
    </Show>
  );
}

/* ---------------- Sheet (slide-in side panel) -------------------------------- */
export interface SheetProps {
  open?: boolean;
  onClose?: () => void;
  title?: string;
  side?: 'left' | 'right';
  footer?: JSX.Element;
  children?: JSX.Element;
}

export function Sheet(props: SheetProps): JSX.Element {
  let backdrop: HTMLDivElement | undefined;
  let panel: HTMLDivElement | undefined;
  useOverlay({
    open: () => !!props.open,
    surface: () => panel,
    backdrop: () => backdrop,
    onDismiss: () => props.onClose?.(),
  });
  return (
    <Show when={props.open}>
      <OverlayPortal>
        <div ref={backdrop} class="fsheet" classList={{ 'is-left': props.side === 'left' }}>
          <div class="fsheet-scrim" />
          <div ref={panel} class="fsheet-panel" role="dialog" aria-label={props.title}>
            <header class="fsheet-head">
              <h3>{props.title}</h3>
              <button class="ftopbar-icon-btn" aria-label="Close" onClick={() => props.onClose?.()}>
                <XSvg />
              </button>
            </header>
            <div class="fsheet-body">{props.children}</div>
            <Show when={props.footer}>
              <footer class="fsheet-foot">{props.footer}</footer>
            </Show>
          </div>
        </div>
      </OverlayPortal>
    </Show>
  );
}

/* ---------------- Tooltip (CSS-only, text-only) ------------------------------ */
export interface TooltipProps {
  label: string;
  side?: 'top' | 'bottom' | 'left' | 'right';
  children?: JSX.Element;
}

export function Tooltip(props: TooltipProps): JSX.Element {
  const merged = mergeProps({ side: 'top' as const }, props);
  return (
    <span class="ftip" data-tip={merged.label} data-side={merged.side}>
      {merged.children}
    </span>
  );
}

/* ---------------- Popover --------------------------------------------------- */
export interface PopoverProps {
  label?: JSX.Element;
  icon?: IconComponent;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: ControlSize;
  align?: 'start' | 'end';
  width?: string;
  children?: JSX.Element;
}

export function Popover(props: PopoverProps): JSX.Element {
  const merged = mergeProps({ variant: 'secondary' as const, size: 'md' as const, align: 'start' as const }, props);
  const [open, setOpen] = createSignal(false);
  let root!: HTMLDivElement;
  useOverlay({ open, surface: () => root, onDismiss: () => setOpen(false) });
  return (
    <div class="fpopover" ref={root}>
      <Button variant={merged.variant} size={merged.size} icon={merged.icon}
              aria-haspopup="dialog" aria-expanded={open()}
              onClick={() => setOpen((o) => !o)}>
        {merged.label}
      </Button>
      <Show when={open()}>
        <div class="fpop fpopover-pop" classList={{ 'is-end': merged.align === 'end' }}
             style={merged.width ? { width: merged.width } : undefined}>
          {merged.children}
        </div>
      </Show>
    </div>
  );
}

/* ---------------- DropdownMenu ---------------------------------------------- */
export interface DropdownMenuProps {
  items: MenuItem[];
  label?: JSX.Element;
  icon?: IconComponent;
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: ControlSize;
  align?: 'start' | 'end';
}

export function DropdownMenu(props: DropdownMenuProps): JSX.Element {
  const merged = mergeProps({ variant: 'secondary' as const, size: 'md' as const, align: 'start' as const }, props);
  const [open, setOpen] = createSignal(false);
  const [activeIdx, setActiveIdx] = createSignal(-1);
  let root!: HTMLDivElement;
  useOverlay({ open, surface: () => root, onDismiss: () => setOpen(false) });

  const selectable = (i: number) => {
    const it = merged.items[i];
    return !!it && !it.separator && !it.disabled;
  };
  const move = (dir: number) => {
    const n = merged.items.length;
    let i = activeIdx();
    for (let step = 0; step < n; step++) {
      i = (i + dir + n) % n;
      if (selectable(i)) { setActiveIdx(i); return; }
    }
  };
  const commit = (i: number) => {
    if (!selectable(i)) return;
    setOpen(false);
    merged.items[i]?.onSelect?.();
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (!open()) {
      if (['ArrowDown', 'Enter', ' '].includes(e.key)) { e.preventDefault(); setActiveIdx(-1); setOpen(true); move(1); }
      return;
    }
    if (e.key === 'ArrowDown') { e.preventDefault(); move(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); move(-1); }
    else if (e.key === 'Home') { e.preventDefault(); setActiveIdx(-1); move(1); }
    else if (e.key === 'End') { e.preventDefault(); setActiveIdx(merged.items.length); move(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); commit(activeIdx()); }
  };

  return (
    <div class="fmenu" ref={root}>
      <Button variant={merged.variant} size={merged.size} icon={merged.icon}
              aria-haspopup="menu" aria-expanded={open()}
              onClick={() => setOpen((o) => !o)} onKeyDown={onKeyDown}>
        {merged.label}
      </Button>
      <Show when={open()}>
        <div class="fpop fmenu-pop" role="menu" classList={{ 'is-end': merged.align === 'end' }}>
          <MenuList items={merged.items} activeIdx={activeIdx} setActiveIdx={setActiveIdx} onCommit={commit} />
        </div>
      </Show>
    </div>
  );
}

/* ---------------- ContextMenu ----------------------------------------------- */
/* Wrap any surface; right-click opens the menu at the cursor. */
export interface ContextMenuProps {
  items: MenuItem[];
  children?: JSX.Element;
}

export function ContextMenu(props: ContextMenuProps): JSX.Element {
  const [pos, setPos] = createSignal<{ x: number; y: number } | null>(null);
  const [activeIdx, setActiveIdx] = createSignal(-1);
  let root!: HTMLDivElement;
  useOverlay({ open: () => !!pos(), surface: () => root, onDismiss: () => setPos(null) });
  const commit = (i: number) => {
    const it = props.items[i];
    if (!it || it.separator || it.disabled) return;
    setPos(null);
    it.onSelect?.();
  };
  return (
    <div class="fctx" ref={root}
         onContextMenu={(e) => {
           e.preventDefault();
           const r = root.getBoundingClientRect();
           setActiveIdx(-1);
           setPos({ x: e.clientX - r.left, y: e.clientY - r.top });
         }}>
      {props.children}
      <Show when={pos()}>
        <div class="fpop fmenu-pop" role="menu"
             style={{ top: `${pos()!.y}px`, left: `${pos()!.x}px` }}>
          <MenuList items={props.items} activeIdx={activeIdx} setActiveIdx={setActiveIdx} onCommit={commit} />
        </div>
      </Show>
    </div>
  );
}

/* ---------------- Command palette ------------------------------------------- */
/* Controlled. Bind the hotkey consumer-side:
   (e.metaKey || e.ctrlKey) && e.key === 'k'  ->  open. */
export interface CommandProps {
  open?: boolean;
  onClose?: () => void;
  items: CommandItem[];
  placeholder?: string;
}

export function Command(props: CommandProps): JSX.Element {
  const [query, setQuery] = createSignal('');
  const [activeIdx, setActiveIdx] = createSignal(0);
  let input: HTMLInputElement | undefined;
  let backdrop: HTMLDivElement | undefined;
  let panel: HTMLDivElement | undefined;

  const close = () => { props.onClose?.(); setQuery(''); };
  useOverlay({
    open: () => !!props.open,
    surface: () => panel,
    backdrop: () => backdrop,
    onDismiss: close,
  });

  const filtered = () => {
    const q = query().toLowerCase();
    return (props.items ?? []).filter((it) => it.label.toLowerCase().includes(q));
  };
  const grouped = () => {
    const out: { group: string; items: CommandItem[] }[] = [];
    for (const it of filtered()) {
      let g = out.find((x) => x.group === (it.group ?? ''));
      if (!g) out.push((g = { group: it.group ?? '', items: [] }));
      g.items.push(it);
    }
    return out;
  };
  const flatIndex = (item: CommandItem) => filtered().indexOf(item);
  const commit = (item: CommandItem | undefined) => {
    if (!item) return;
    props.onClose?.();
    setQuery('');
    item.onSelect?.();
  };
  const onKeyDown = (e: KeyboardEvent) => {
    const n = filtered().length;
    if (e.key === 'ArrowDown') { e.preventDefault(); setActiveIdx((i) => (i + 1) % Math.max(1, n)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setActiveIdx((i) => (i - 1 + n) % Math.max(1, n)); }
    else if (e.key === 'Enter') { e.preventDefault(); commit(filtered()[activeIdx()]); }
  };
  createEffect(() => { if (props.open) { setActiveIdx(0); queueMicrotask(() => input?.focus()); } });

  return (
    <Show when={props.open}>
      <OverlayPortal>
        <div ref={backdrop} class="fcmd">
          <div ref={panel} class="fcmd-panel" role="dialog" aria-label="Command palette">
            <div class="fcmd-input">
              <SearchSvg />
              <input ref={input} placeholder={props.placeholder ?? 'Type a command…'}
                     value={query()}
                     onInput={(e) => { setQuery(e.currentTarget.value); setActiveIdx(0); }}
                     onKeyDown={onKeyDown} />
              <Kbd>esc</Kbd>
            </div>
            <div class="fcmd-list">
              <Show when={filtered().length} fallback={<div class="fcmd-empty">No results</div>}>
                <For each={grouped()}>
                  {(g) => (
                    <>
                      <Show when={g.group}>
                        <div class="fcmd-group">{g.group}</div>
                      </Show>
                      <For each={g.items}>
                        {(item) => (
                          <button type="button" class="fcmd-item"
                                  classList={{ 'is-active': flatIndex(item) === activeIdx() }}
                                  onPointerEnter={() => setActiveIdx(flatIndex(item))}
                                  onClick={() => commit(item)}>
                            <Show when={item.icon}>
                              <Icon of={item.icon!} size={14} />
                            </Show>
                            <span class="fmenu-label">{item.label}</span>
                            <Show when={item.kbd}>
                              <span class="fmenu-kbd">{item.kbd}</span>
                            </Show>
                          </button>
                        )}
                      </For>
                    </>
                  )}
                </For>
              </Show>
            </div>
          </div>
        </div>
      </OverlayPortal>
    </Show>
  );
}
