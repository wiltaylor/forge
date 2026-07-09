import { For, Show } from 'solid-js';
import type { Accessor, Setter } from 'solid-js';
import { Icon } from '../primitives';
import type { MenuItem } from '../types';

export interface MenuListProps {
  items: MenuItem[];
  activeIdx: Accessor<number>;
  setActiveIdx: Setter<number>;
  onCommit: (idx: number) => void;
}

/** Shared menu body for DropdownMenu / ContextMenu. */
export function MenuList(props: MenuListProps) {
  return (
    <For each={props.items}>
      {(item, i) => (
        <Show when={!item.separator} fallback={<div class="fmenu-sep" role="separator" />}>
          <button type="button" class="fmenu-item" role="menuitem" disabled={item.disabled}
                  classList={{
                    'is-active': i() === props.activeIdx(),
                    'is-danger': !!item.danger,
                    'is-disabled': !!item.disabled,
                  }}
                  onPointerEnter={() => !item.disabled && props.setActiveIdx(i())}
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
