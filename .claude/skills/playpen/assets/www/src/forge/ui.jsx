/* Forge UI primitives — SolidJS port of ui_kits/console/ui.jsx.
   Requires console.css + colors_and_type.css to be imported by the app.
   Icons come from lucide-solid: pass the imported icon component via the
   `icon` prop, e.g. icon={Terminal}. All icons render at 1.5px stroke. */

import { Show, For, Index, createSignal, createEffect, createUniqueId, onCleanup, mergeProps, splitProps } from 'solid-js';
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

/* ===========================================================================
   Form controls — Checkbox / Toggle / Radio hide a native input (a11y + form
   semantics) and draw the control with console.css classes.
   =========================================================================== */

const CheckMark = () => (
  <svg class="fcheck-mark" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M4 12l5 5L20 6" />
  </svg>
);
const CheckDash = () => (
  <svg class="fcheck-dash" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="3" stroke-linecap="round" aria-hidden="true" style={{ position: 'absolute' }}>
    <line x1="5" y1="12" x2="19" y2="12" />
  </svg>
);
const ChevronDown = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
       stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M6 9l6 6 6-6" />
  </svg>
);

/* ---------------- Checkbox ------------------------------------------------- */
export function Checkbox(props) {
  const [local, rest] = splitProps(props, ['checked', 'onChange', 'indeterminate', 'children']);
  let input;
  createEffect(() => { if (input) input.indeterminate = !!local.indeterminate; });
  return (
    <label class="fcheck">
      <input ref={input} type="checkbox" checked={local.checked}
             onInput={(e) => local.onChange?.(e.currentTarget.checked)} {...rest} />
      <span class="fcheck-box"><CheckMark /><CheckDash /></span>
      <Show when={local.children}>
        <span class="fcheck-label">{local.children}</span>
      </Show>
    </label>
  );
}

/* ---------------- Toggle (switch) ------------------------------------------ */
export function Toggle(props) {
  const [local, rest] = splitProps(props, ['checked', 'onChange', 'children']);
  return (
    <label class="ftoggle">
      <input type="checkbox" role="switch" checked={local.checked}
             onInput={(e) => local.onChange?.(e.currentTarget.checked)} {...rest} />
      <span class="ftoggle-track"><span class="ftoggle-knob" /></span>
      <Show when={local.children}>
        <span class="ftoggle-label">{local.children}</span>
      </Show>
    </label>
  );
}

/* ---------------- Radio ---------------------------------------------------- */
export function Radio(props) {
  const [local, rest] = splitProps(props, ['value', 'checked', 'onChange', 'children']);
  return (
    <label class="fradio">
      <input type="radio" value={local.value} checked={local.checked}
             onInput={() => local.onChange?.(local.value)} {...rest} />
      <span class="fradio-dot" />
      <Show when={local.children}>
        <span class="fradio-label">{local.children}</span>
      </Show>
    </label>
  );
}

export function RadioGroup(props) {
  const merged = mergeProps({ name: createUniqueId() }, props);
  return (
    <div class="ffield">
      <Show when={merged.label}>
        <span class="ffield-label">{merged.label}</span>
      </Show>
      <div class="fradio-group" classList={{ 'is-row': !!merged.row }} role="radiogroup">
        <For each={merged.options}>
          {(opt) => (
            <Radio name={merged.name} value={opt.value} checked={merged.value === opt.value}
                   disabled={opt.disabled} onChange={(v) => merged.onChange?.(v)}>
              {opt.label}
            </Radio>
          )}
        </For>
      </div>
    </div>
  );
}

/* ---------------- Select (custom popover dropdown) -------------------------- */
export function Select(props) {
  const [local, rest] = splitProps(props,
    ['options', 'value', 'onChange', 'placeholder', 'label', 'help', 'error', 'children']);
  const [open, setOpen] = createSignal(false);
  const [activeIdx, setActiveIdx] = createSignal(-1);
  let root;

  const selected = () => local.options?.find((o) => o.value === local.value);
  const enabledIdx = (from, dir) => {
    const opts = local.options ?? [];
    for (let i = from; i >= 0 && i < opts.length; i += dir) if (!opts[i].disabled) return i;
    return -1;
  };
  const openAt = () => {
    const cur = local.options?.findIndex((o) => o.value === local.value) ?? -1;
    setActiveIdx(cur >= 0 ? cur : enabledIdx(0, 1));
    setOpen(true);
  };
  const commit = (idx) => {
    const opt = local.options?.[idx];
    if (opt && !opt.disabled) { local.onChange?.(opt.value); setOpen(false); }
  };
  const onKeyDown = (e) => {
    if (!open()) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) { e.preventDefault(); openAt(); }
      return;
    }
    if (e.key === 'Escape') setOpen(false);
    else if (e.key === 'ArrowDown') { e.preventDefault(); setActiveIdx((i) => enabledIdx(Math.min(i + 1, local.options.length - 1), 1) ?? i); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setActiveIdx((i) => enabledIdx(Math.max(i - 1, 0), -1) ?? i); }
    else if (e.key === 'Home') { e.preventDefault(); setActiveIdx(enabledIdx(0, 1)); }
    else if (e.key === 'End') { e.preventDefault(); setActiveIdx(enabledIdx(local.options.length - 1, -1)); }
    else if (e.key === 'Enter') { e.preventDefault(); commit(activeIdx()); }
  };
  createEffect(() => {
    if (!open()) return;
    const onDown = (e) => { if (!root.contains(e.target)) setOpen(false); };
    document.addEventListener('pointerdown', onDown);
    onCleanup(() => document.removeEventListener('pointerdown', onDown));
  });

  return (
    <div class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <div class="fselect" classList={{ 'is-open': open() }} ref={root}>
        <button type="button" class="fselect-btn" aria-haspopup="listbox" aria-expanded={open()}
                onClick={() => (open() ? setOpen(false) : openAt())} onKeyDown={onKeyDown} {...rest}>
          <span class="fselect-value" classList={{ 'is-placeholder': !selected() }}>
            {selected()?.label ?? local.placeholder ?? 'Select…'}
          </span>
          <ChevronDown />
        </button>
        <Show when={open()}>
          <div class="fselect-pop" role="listbox">
            <For each={local.options}>
              {(opt, i) => (
                <div class="fselect-opt" role="option" aria-selected={opt.value === local.value}
                     classList={{
                       'is-active': i() === activeIdx(),
                       'is-selected': opt.value === local.value,
                       'is-disabled': !!opt.disabled,
                     }}
                     onPointerEnter={() => !opt.disabled && setActiveIdx(i())}
                     onClick={() => commit(i())}>
                  {opt.label}
                  <Show when={opt.value === local.value}>
                    <span class="fselect-check"><CheckMark /></span>
                  </Show>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
      <Show when={local.help}>
        <span class="ffield-help" classList={{ 'is-error': !!local.error }}>{local.help}</span>
      </Show>
    </div>
  );
}

/* ---------------- ListBox --------------------------------------------------- */
/* Single-select: value/onChange(value). Multi: multiple + values/onChange(values). */
export function ListBox(props) {
  const [local, rest] = splitProps(props,
    ['options', 'value', 'values', 'onChange', 'multiple', 'label']);
  const [activeIdx, setActiveIdx] = createSignal(-1);

  const isSelected = (opt) =>
    local.multiple ? (local.values ?? []).includes(opt.value) : opt.value === local.value;
  const pick = (opt) => {
    if (opt.disabled) return;
    if (local.multiple) {
      const cur = local.values ?? [];
      local.onChange?.(cur.includes(opt.value) ? cur.filter((v) => v !== opt.value) : [...cur, opt.value]);
    } else {
      local.onChange?.(opt.value);
    }
  };
  const onKeyDown = (e) => {
    const opts = local.options ?? [];
    if (e.key === 'ArrowDown') { e.preventDefault(); setActiveIdx((i) => Math.min(i + 1, opts.length - 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setActiveIdx((i) => Math.max(i - 1, 0)); }
    else if (e.key === 'Home') { e.preventDefault(); setActiveIdx(0); }
    else if (e.key === 'End') { e.preventDefault(); setActiveIdx(opts.length - 1); }
    else if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (opts[activeIdx()]) pick(opts[activeIdx()]); }
  };

  return (
    <div class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <div class="flistbox" role="listbox" tabindex="0" aria-multiselectable={!!local.multiple}
           onKeyDown={onKeyDown} {...rest}>
        <For each={local.options}>
          {(opt, i) => (
            <div class="flistbox-opt" role="option" aria-selected={isSelected(opt)}
                 classList={{ 'is-selected': isSelected(opt), 'is-active': i() === activeIdx(), 'is-disabled': !!opt.disabled }}
                 onClick={() => { setActiveIdx(i()); pick(opt); }}>
              <Show when={local.multiple}>
                <span class="flistbox-check"><CheckMark /></span>
              </Show>
              {opt.label}
            </div>
          )}
        </For>
      </div>
    </div>
  );
}

/* ---------------- Progress -------------------------------------------------- */
/* Thin bar — the default Forge loading treatment. */
export function Progress(props) {
  const merged = mergeProps({ tone: 'accent', value: 0 }, props);
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
export function Spinner(props) {
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
