import { For, Show } from 'solid-js';
import type { JSX } from 'solid-js';
import { Icon } from '../primitives';
import { createRoving } from '../overlay';
import type { Roving } from '../overlay';
import type { MenuItem } from '../types';

/** Can this item be picked? A separator and a disabled item cannot. */
export function menuSelectable(items: MenuItem[], i: number): boolean {
  const it = items[i];
  return !!it && !it.separator && !it.disabled;
}

/** The roving index of a menu: it moves over the items a click could pick. */
export function menuRoving(items: () => MenuItem[]): Roving {
  return createRoving({ count: () => items().length, enabled: (i) => menuSelectable(items(), i) });
}

export interface MenuListProps {
  items: MenuItem[];
  /** The menu's roving index: which item is active, and how a pointer moves it. */
  roving: Roving;
  onCommit: (idx: number) => void;
}

/** Shared menu body for DropdownMenu / ContextMenu. */
export function MenuList(props: MenuListProps): JSX.Element {
  return (
    <For each={props.items}>
      {(item, i) => (
        <Show when={!item.separator} fallback={<div class="fmenu-sep" role="separator" />}>
          <button type="button" class="fmenu-item" role="menuitem" disabled={item.disabled}
                  id={props.roving.itemId(i())}
                  classList={{
                    'is-active': i() === props.roving.active(),
                    'is-danger': !!item.danger,
                    'is-disabled': !!item.disabled,
                  }}
                  onPointerEnter={() => !item.disabled && props.roving.setActive(i())}
                  onClick={() => !item.disabled && props.onCommit(i())}>
            <Show when={item.icon}>
              <Icon of={item.icon!} size={14} />
            </Show>
            <span class="fmenu-label">{item.label}</span>
            <Show when={item.kbd}>
              <span class="fmenu-kbd">{item.kbd}</span>
            </Show>
          </button>
        </Show>
      )}
    </For>
  );
}
