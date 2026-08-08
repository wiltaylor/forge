/* The active-descendant announcement: as the roving index moves, the element
   that holds focus points at the active option through
   `aria-activedescendant`, and the option carries the id the pointer needs.
   The assertions read the way a screen reader does — resolve the attribute to
   an element, and look at what that element says. */
import { describe, expect, it } from 'vitest';
import type { JSX } from 'solid-js';
import { fireEvent, render, screen } from '@solidjs/testing-library';
import { Command, DropdownMenu } from '../src/overlays';
import { Combobox, ListBox, Select } from '../src/forms';

const LABELS = ['Alpha', 'Beta', 'Gamma'];
const options = LABELS.map((label) => ({ value: label, label }));
const items = LABELS.map((label) => ({ label }));

interface Widget {
  name: string;
  /** Mount it over Alpha, Beta and Gamma. */
  mount: () => JSX.Element;
  /** Open it, if it opens, and return the element that holds the keys. */
  open: () => HTMLElement;
}

const widgets: Widget[] = [
  {
    name: 'Select',
    mount: () => <Select options={options} label="Kind" />,
    open: () => {
      const trigger = screen.getByRole('button', { name: 'Select…' });
      fireEvent.click(trigger);
      return trigger;
    },
  },
  {
    name: 'ListBox',
    mount: () => <ListBox options={options} label="Kind" />,
    open: () => screen.getByRole('listbox'),
  },
  {
    name: 'Combobox',
    mount: () => <Combobox options={options} label="Kind" />,
    open: () => {
      const input = screen.getByRole('combobox');
      fireEvent.focus(input);
      return input;
    },
  },
  {
    name: 'DropdownMenu',
    mount: () => <DropdownMenu label="Actions" items={items} />,
    open: () => {
      const trigger = screen.getByRole('button', { name: 'Actions' });
      fireEvent.click(trigger);
      return trigger;
    },
  },
  {
    name: 'Command',
    mount: () => <Command open items={items} />,
    open: () => screen.getByRole('textbox'),
  },
];

/** What the attribute announces: the text of the element it points at. */
function announced(target: HTMLElement): string | undefined {
  const id = target.getAttribute('aria-activedescendant');
  if (!id) return undefined;
  const option = document.getElementById(id);
  expect(option, `aria-activedescendant points at "${id}", which is not in the document`).not.toBeNull();
  return option!.textContent ?? undefined;
}

for (const widget of widgets) {
  describe(`the announcement in a ${widget.name}`, () => {
    it('follows the index through arrow movement, Home and End', () => {
      render(() => widget.mount());
      const target = widget.open();

      fireEvent.keyDown(target, { key: 'Home' });
      expect(announced(target)).toBe('Alpha');

      fireEvent.keyDown(target, { key: 'ArrowDown' });
      expect(announced(target)).toBe('Beta');

      fireEvent.keyDown(target, { key: 'ArrowUp' });
      expect(announced(target)).toBe('Alpha');

      fireEvent.keyDown(target, { key: 'End' });
      expect(announced(target)).toBe('Gamma');
    });
  });
}

describe('the announcement when no option is active', () => {
  it('says nothing for a ListBox the keys have not entered', () => {
    render(() => <ListBox options={options} label="Kind" />);
    expect(screen.getByRole('listbox').getAttribute('aria-activedescendant')).toBeNull();
  });

  it('says nothing for a closed DropdownMenu after use', () => {
    render(() => <DropdownMenu label="Actions" items={items} />);
    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.click(trigger);
    fireEvent.keyDown(trigger, { key: 'Home' });
    fireEvent.keyDown(trigger, { key: 'Enter' });
    expect(trigger.getAttribute('aria-activedescendant')).toBeNull();
  });
});
