import { For, Show, createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { CalendarSvg, ChevronDown, ChevronLeftSvg, ChevronRightSvg } from './internal/icons';
import { useDismiss } from './internal/dismiss';
import { MONTHS, isoOf } from './internal/date';

/* Dates are ISO YYYY-MM-DD strings; weeks start Monday; min/max compare
   lexicographically (safe for ISO strings). */

export interface CalendarProps {
  value?: string;
  onChange?: (iso: string) => void;
  min?: string;
  max?: string;
}

export function Calendar(props: CalendarProps) {
  const initial = () => {
    const v = props.value ? new Date(`${props.value}T00:00:00`) : new Date();
    return { y: v.getFullYear(), m: v.getMonth() };
  };
  const [view, setView] = createSignal(initial());

  const cells = () => {
    const { y, m } = view();
    const first = new Date(y, m, 1);
    const lead = (first.getDay() + 6) % 7;  // Monday-start offset
    const start = new Date(y, m, 1 - lead);
    return Array.from({ length: 42 }, (_, i) => {
      const d = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
      return { iso: isoOf(d.getFullYear(), d.getMonth(), d.getDate()), out: d.getMonth() !== m, day: d.getDate() };
    });
  };
  const nav = (dir: number) => setView(({ y, m }) => {
    const d = new Date(y, m + dir, 1);
    return { y: d.getFullYear(), m: d.getMonth() };
  });
  const today = isoOf(new Date().getFullYear(), new Date().getMonth(), new Date().getDate());
  const disabled = (iso: string) => (props.min != null && iso < props.min) || (props.max != null && iso > props.max);

  return (
    <div class="fcal">
      <div class="fcal-head">
        <button type="button" class="fcal-nav" aria-label="Previous month" onClick={() => nav(-1)}>
          <ChevronLeftSvg />
        </button>
        <span class="fcal-title">{MONTHS[view().m]} {view().y}</span>
        <button type="button" class="fcal-nav" aria-label="Next month" onClick={() => nav(1)}>
          <ChevronRightSvg />
        </button>
      </div>
      <div class="fcal-dow">
        <For each={['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su']}>{(d) => <span>{d}</span>}</For>
      </div>
      <div class="fcal-grid">
        <For each={cells()}>
          {(c) => (
            <button type="button" class="fcal-day" disabled={disabled(c.iso)}
                    classList={{ 'is-out': c.out, 'is-today': c.iso === today, 'is-selected': c.iso === props.value }}
                    onClick={() => props.onChange?.(c.iso)}>
              {c.day}
            </button>
          )}
        </For>
      </div>
    </div>
  );
}

export interface DatePickerProps extends CalendarProps {
  label?: JSX.Element;
  placeholder?: string;
  help?: JSX.Element;
  error?: boolean;
  disabled?: boolean;
}

export function DatePicker(props: DatePickerProps) {
  const [open, setOpen] = createSignal(false);
  let root!: HTMLDivElement;
  useDismiss(open, () => setOpen(false), () => root);
  return (
    <div class="ffield">
      <Show when={props.label}>
        <span class="ffield-label">{props.label}</span>
      </Show>
      <div class="fdate" ref={root}>
        <button type="button" class="fselect-btn" disabled={props.disabled}
                aria-haspopup="dialog" aria-expanded={open()}
                onClick={() => setOpen((o) => !o)}>
          <CalendarSvg />
          <span class="fselect-value" classList={{ 'is-placeholder': !props.value }}>
            {props.value ?? props.placeholder ?? 'Pick a date…'}
          </span>
          <ChevronDown />
        </button>
        <Show when={open()}>
          <div class="fpop fdate-pop">
            <Calendar value={props.value} min={props.min} max={props.max}
                      onChange={(iso) => { props.onChange?.(iso); setOpen(false); }} />
          </div>
        </Show>
      </div>
      <Show when={props.help}>
        <span class="ffield-help" classList={{ 'is-error': !!props.error }}>{props.help}</span>
      </Show>
    </div>
  );
}
