import { For, Show, createEffect, createSignal, createUniqueId, mergeProps, splitProps } from 'solid-js';
import type { JSX } from 'solid-js';
import { CheckDash, CheckMark, ChevronDown, SearchSvg } from './internal/icons';
import { createRoving, useOverlay } from './overlay';
import type { Roving } from './overlay';
import { Icon } from './primitives';
import type { IconComponent, Option } from './types';

/** The roving index of an option list: it moves over the options a user can pick. */
function optionRoving<T>(options: () => Option<T>[] | undefined): Roving {
  return createRoving({
    count: () => options()?.length ?? 0,
    enabled: (i) => !options()?.[i]?.disabled,
  });
}

/* ---------------- Input ---------------------------------------------------- */
export interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  icon?: IconComponent;
  error?: boolean;
  label?: JSX.Element;
  help?: JSX.Element;
}

export function Input(props: InputProps): JSX.Element {
  const [local, rest] = splitProps(props, ['icon', 'error', 'label', 'help']);
  return (
    <label class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <span class="ffield-input" classList={{ 'is-error': !!local.error }}>
        <Show when={local.icon}>
          <Icon of={local.icon!} size={14} />
        </Show>
        <input {...rest} />
      </span>
      <Show when={local.help}>
        <span class="ffield-help" classList={{ 'is-error': !!local.error }}>{local.help}</span>
      </Show>
    </label>
  );
}

/* ---------------- Textarea -------------------------------------------------- */
export interface TextareaProps extends JSX.TextareaHTMLAttributes<HTMLTextAreaElement> {
  error?: boolean;
  label?: JSX.Element;
  help?: JSX.Element;
}

export function Textarea(props: TextareaProps): JSX.Element {
  const [local, rest] = splitProps(props, ['label', 'help', 'error']);
  return (
    <label class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <span class="ffield-area" classList={{ 'is-error': !!local.error }}>
        <textarea {...rest} />
      </span>
      <Show when={local.help}>
        <span class="ffield-help" classList={{ 'is-error': !!local.error }}>{local.help}</span>
      </Show>
    </label>
  );
}

/* ---------------- Checkbox ------------------------------------------------- */
export interface CheckboxProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'onInput' | 'children'> {
  checked?: boolean;
  indeterminate?: boolean;
  onChange?: (checked: boolean) => void;
  children?: JSX.Element;
}

export function Checkbox(props: CheckboxProps): JSX.Element {
  const [local, rest] = splitProps(props, ['checked', 'onChange', 'indeterminate', 'children']);
  let input: HTMLInputElement | undefined;
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
export interface ToggleProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'onInput' | 'children'> {
  checked?: boolean;
  onChange?: (checked: boolean) => void;
  children?: JSX.Element;
}

export function Toggle(props: ToggleProps): JSX.Element {
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
export interface RadioProps<T = string> extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'onInput' | 'children' | 'value'> {
  value: T;
  checked?: boolean;
  onChange?: (value: T) => void;
  children?: JSX.Element;
}

export function Radio<T = string>(props: RadioProps<T>): JSX.Element {
  const [local, rest] = splitProps(props, ['value', 'checked', 'onChange', 'children']);
  return (
    <label class="fradio">
      <input type="radio" value={String(local.value)} checked={local.checked}
             onInput={() => local.onChange?.(local.value)} {...rest} />
      <span class="fradio-dot" />
      <Show when={local.children}>
        <span class="fradio-label">{local.children}</span>
      </Show>
    </label>
  );
}

export interface RadioGroupProps<T = string> {
  options: Option<T>[];
  value?: T;
  onChange?: (value: T) => void;
  label?: JSX.Element;
  name?: string;
  row?: boolean;
}

export function RadioGroup<T = string>(props: RadioGroupProps<T>): JSX.Element {
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
export interface SelectProps<T = string> extends Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, 'onChange' | 'value'> {
  options: Option<T>[];
  value?: T;
  onChange?: (value: T) => void;
  placeholder?: string;
  label?: JSX.Element;
  help?: JSX.Element;
  error?: boolean;
}

export function Select<T = string>(props: SelectProps<T>): JSX.Element {
  const [local, rest] = splitProps(props,
    ['options', 'value', 'onChange', 'placeholder', 'label', 'help', 'error', 'children']);
  const [open, setOpen] = createSignal(false);
  const roving = optionRoving(() => local.options);
  let root!: HTMLDivElement;
  useOverlay({ open, surface: () => root, onDismiss: () => setOpen(false) });

  const selected = () => local.options?.find((o) => o.value === local.value);
  const openAt = () => {
    const cur = local.options?.findIndex((o) => o.value === local.value) ?? -1;
    if (cur >= 0) roving.setActive(cur);
    else roving.first();
    setOpen(true);
  };
  const commit = (idx: number) => {
    const opt = local.options?.[idx];
    if (opt && !opt.disabled) { local.onChange?.(opt.value); setOpen(false); }
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (!open()) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) { e.preventDefault(); openAt(); }
      return;
    }
    if (roving.onKeyDown(e)) return;
    if (e.key === 'Enter') { e.preventDefault(); commit(roving.active()); }
  };

  return (
    <div class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <div class="fselect" classList={{ 'is-open': open() }} ref={root}>
        <button type="button" class="fselect-btn" aria-haspopup="listbox" aria-expanded={open()}
                aria-activedescendant={open() ? roving.activeId() : undefined}
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
                     id={roving.itemId(i())}
                     classList={{
                       'is-active': i() === roving.active(),
                       'is-selected': opt.value === local.value,
                       'is-disabled': !!opt.disabled,
                     }}
                     onPointerEnter={() => !opt.disabled && roving.setActive(i())}
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
export interface ListBoxProps<T = string> {
  options: Option<T>[];
  value?: T;
  values?: T[];
  multiple?: boolean;
  onChange?: (value: T & T[]) => void;
  label?: JSX.Element;
}

export function ListBox<T = string>(props: ListBoxProps<T> & Omit<JSX.HTMLAttributes<HTMLDivElement>, 'onChange'>): JSX.Element {
  const [local, rest] = splitProps(props,
    ['options', 'value', 'values', 'onChange', 'multiple', 'label']);
  const roving = optionRoving(() => local.options);
  const emit = (v: T | T[]) => (local.onChange as ((v: T | T[]) => void) | undefined)?.(v);

  const isSelected = (opt: Option<T>) =>
    local.multiple ? (local.values ?? []).includes(opt.value) : opt.value === local.value;
  const pick = (opt: Option<T>) => {
    if (opt.disabled) return;
    if (local.multiple) {
      const cur = local.values ?? [];
      emit(cur.includes(opt.value) ? cur.filter((v) => v !== opt.value) : [...cur, opt.value]);
    } else {
      emit(opt.value);
    }
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (roving.onKeyDown(e)) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const opt = (local.options ?? [])[roving.active()];
      if (opt) pick(opt);
    }
  };

  return (
    <div class="ffield">
      <Show when={local.label}>
        <span class="ffield-label">{local.label}</span>
      </Show>
      <div class="flistbox" role="listbox" tabindex="0" aria-multiselectable={!!local.multiple}
           aria-activedescendant={roving.activeId()}
           onKeyDown={onKeyDown} {...rest}>
        <For each={local.options}>
          {(opt, i) => (
            <div class="flistbox-opt" role="option" aria-selected={isSelected(opt)}
                 id={roving.itemId(i())}
                 classList={{ 'is-selected': isSelected(opt), 'is-active': i() === roving.active(), 'is-disabled': !!opt.disabled }}
                 onClick={() => { roving.setActive(i()); pick(opt); }}>
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

/* ---------------- Slider ---------------------------------------------------- */
export interface SliderProps extends Omit<JSX.InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'onInput' | 'value' | 'min' | 'max' | 'step'> {
  value: number;
  onChange?: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  label?: JSX.Element;
  showValue?: boolean;
}

export function Slider(props: SliderProps): JSX.Element {
  const merged = mergeProps({ min: 0, max: 100, step: 1 }, props);
  const [local, rest] = splitProps(merged, ['value', 'onChange', 'label', 'showValue', 'min', 'max', 'step']);
  const pct = () => ((local.value - local.min) / (local.max - local.min)) * 100;
  return (
    <div class="ffield">
      <Show when={local.label || local.showValue}>
        <div class="fslider-head">
          <span class="ffield-label">{local.label}</span>
          <Show when={local.showValue}>
            <span class="fslider-value">{local.value}</span>
          </Show>
        </div>
      </Show>
      <input type="range" class="fslider" min={local.min} max={local.max} step={local.step}
             value={local.value} style={{ '--fslider-fill': `${pct()}%` }}
             onInput={(e) => local.onChange?.(Number(e.currentTarget.value))} {...rest} />
    </div>
  );
}

/* ---------------- ToggleGroup (segmented) ----------------------------------- */
export interface ToggleGroupProps<T = string> {
  options: Option<T>[];
  value?: T;
  onChange?: (value: T) => void;
}

export function ToggleGroup<T = string>(props: ToggleGroupProps<T>): JSX.Element {
  return (
    <div class="fseg" role="radiogroup">
      <For each={props.options}>
        {(opt) => (
          <button type="button" class="fseg-btn" role="radio" disabled={opt.disabled}
                  aria-checked={props.value === opt.value}
                  classList={{ 'is-active': props.value === opt.value }}
                  onClick={() => props.onChange?.(opt.value)}>
            <Show when={opt.icon}>
              <Icon of={opt.icon!} size={13} />
            </Show>
            {opt.label}
          </button>
        )}
      </For>
    </div>
  );
}

/* ---------------- Combobox (searchable select) ------------------------------ */
export interface ComboboxProps<T = string> {
  options: Option<T>[];
  value?: T;
  onChange?: (value: T) => void;
  label?: JSX.Element;
  placeholder?: string;
  help?: JSX.Element;
  error?: boolean;
  disabled?: boolean;
  emptyText?: string;
}

export function Combobox<T = string>(props: ComboboxProps<T>): JSX.Element {
  const [open, setOpen] = createSignal(false);
  const [query, setQuery] = createSignal<string | null>(null);  // null = show selected label
  let root!: HTMLDivElement;
  let input!: HTMLInputElement;
  useOverlay({ open, surface: () => root, onDismiss: () => { setOpen(false); setQuery(null); } });

  const selected = () => props.options?.find((o) => o.value === props.value);
  const filtered = () => {
    const q = (query() ?? '').toLowerCase();
    const opts = props.options ?? [];
    return q ? opts.filter((o) => String(o.label).toLowerCase().includes(q)) : opts;
  };
  /* The index moves over what the query left, not over every option. */
  const roving = optionRoving(filtered);
  const commit = (opt: Option<T> | undefined) => {
    if (!opt || opt.disabled) return;
    props.onChange?.(opt.value);
    setOpen(false);
    setQuery(null);
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'ArrowDown') setOpen(true);
    if (roving.onKeyDown(e)) return;
    if (e.key === 'Enter') { e.preventDefault(); commit(filtered()[roving.active()]); }
  };

  return (
    <div class="ffield">
      <Show when={props.label}>
        <span class="ffield-label">{props.label}</span>
      </Show>
      <div class="fcombo" ref={root}>
        <span class="ffield-input" classList={{ 'is-error': !!props.error }}>
          <SearchSvg />
          <input ref={input} role="combobox" aria-expanded={open()} disabled={props.disabled}
                 aria-activedescendant={open() ? roving.activeId() : undefined}
                 placeholder={props.placeholder}
                 value={query() ?? String(selected()?.label ?? '')}
                 onInput={(e) => { setQuery(e.currentTarget.value); setOpen(true); roving.first(); }}
                 onFocus={() => { setOpen(true); input.select(); }}
                 onKeyDown={onKeyDown} />
          <ChevronDown />
        </span>
        <Show when={open()}>
          <div class="fselect-pop" role="listbox">
            <Show when={filtered().length} fallback={<div class="fcmd-empty">{props.emptyText ?? 'No matches'}</div>}>
              <For each={filtered()}>
                {(opt, i) => (
                  <div class="fselect-opt" role="option" aria-selected={opt.value === props.value}
                       id={roving.itemId(i())}
                       classList={{
                         'is-active': i() === roving.active(),
                         'is-selected': opt.value === props.value,
                         'is-disabled': !!opt.disabled,
                       }}
                       onPointerEnter={() => !opt.disabled && roving.setActive(i())}
                       onPointerDown={(e) => e.preventDefault()}
                       onClick={() => commit(opt)}>
                    {opt.label}
                    <Show when={opt.value === props.value}>
                      <span class="fselect-check"><CheckMark /></span>
                    </Show>
                  </div>
                )}
              </For>
            </Show>
          </div>
        </Show>
      </div>
      <Show when={props.help}>
        <span class="ffield-help" classList={{ 'is-error': !!props.error }}>{props.help}</span>
      </Show>
    </div>
  );
}
