/* Schema-driven card controls: maps each KanbanField to the matching
   @forge/ui control, reading from card.data[key] and reporting every edit as
   the FULL next data object through onCardChange — the consumer's reducer is
   a one-liner.

   Known limitation: Select/DatePicker popovers render inline (absolute), so
   they clip against the column body's overflow-y scroll — options stay
   reachable because the open popover extends the scroll extent. The proper
   fix is portal-based popovers in @forge/ui (overlay-mount), not here. */

import { For, Match, Show, Switch } from 'solid-js';
import { Badge, Checkbox, DatePicker, Input, Select, Slider, Textarea, Toggle } from '@forge/ui';
import type { KanbanCard, KanbanField } from './board';

export interface CardFieldsProps {
  card: KanbanCard;
  fields: KanbanField[];
  onCardChange?: (cardId: string, data: Record<string, unknown>) => void;
}

export function CardFields(props: CardFieldsProps) {
  const value = (key: string) => props.card.data[key];
  const set = (key: string) => (v: unknown) =>
    props.onCardChange?.(props.card.id, { ...props.card.data, [key]: v });

  return (
    <For each={props.fields}>
      {(field) => (
        <Switch>
          <Match when={field.type === 'text' && field}>
            {(f) => (
              <Input label={f().label} placeholder={f().placeholder}
                     value={String(value(f().key) ?? '')}
                     onInput={(e) => set(f().key)(e.currentTarget.value)} />
            )}
          </Match>
          <Match when={field.type === 'textarea' && field}>
            {(f) => (
              <Textarea label={f().label} rows={f().rows ?? 2}
                        value={String(value(f().key) ?? '')}
                        onInput={(e) => set(f().key)(e.currentTarget.value)} />
            )}
          </Match>
          <Match when={field.type === 'select' && field}>
            {(f) => (
              <Select options={f().options} placeholder={f().placeholder}
                      label={f().label}
                      value={value(f().key) as string | undefined}
                      onChange={set(f().key)} />
            )}
          </Match>
          <Match when={field.type === 'date' && field}>
            {(f) => (
              <DatePicker label={f().label} min={f().min} max={f().max}
                          value={value(f().key) as string | undefined}
                          onChange={set(f().key)} />
            )}
          </Match>
          <Match when={field.type === 'checkbox' && field}>
            {(f) => (
              <Checkbox checked={Boolean(value(f().key))} onChange={set(f().key)}>
                {f().label}
              </Checkbox>
            )}
          </Match>
          <Match when={field.type === 'toggle' && field}>
            {(f) => (
              <Toggle checked={Boolean(value(f().key))} onChange={set(f().key)}>
                {f().label}
              </Toggle>
            )}
          </Match>
          <Match when={field.type === 'slider' && field}>
            {(f) => (
              <Slider label={f().label} min={f().min} max={f().max} step={f().step}
                      showValue={f().showValue}
                      value={Number(value(f().key) ?? f().min ?? 0)}
                      onChange={set(f().key)} />
            )}
          </Match>
          <Match when={field.type === 'badge' && field}>
            {(f) => (
              <Show when={value(f().key) != null}>
                <div class="fkanban-badge-row">
                  <Show when={f().label}>
                    <span class="ffield-label">{f().label}</span>
                  </Show>
                  <Badge tone={f().tones?.[String(value(f().key))] ?? 'neutral'}>
                    {String(value(f().key))}
                  </Badge>
                </div>
              </Show>
            )}
          </Match>
        </Switch>
      )}
    </For>
  );
}
