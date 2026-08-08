/* The roving index, tested through the five widgets that adapt it: press a
   movement key, press Enter, and assert on the option the widget picked. The
   highlight itself is a class today, so what it commits is the observable part
   of where the index landed. */
import { describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import type { JSX } from 'solid-js';
import { fireEvent, render, screen } from '@solidjs/testing-library';
import { Command, DropdownMenu } from '../src/overlays';
import { Combobox, ListBox, Select } from '../src/forms';

const LABELS = ['Alpha', 'Beta', 'Gamma'];
const options = LABELS.map((label) => ({ value: label, label }));

interface Widget {
  name: string;
  /** Mount it over Alpha, Beta and Gamma, reporting the label it picks. */
  mount: (picked: (label: string) => void) => JSX.Element;
  /** Open it, if it opens, and return the element its keys go to. */
  open: () => HTMLElement;
}

const widgets: Widget[] = [
  {
    name: 'Select',
    mount: (picked) => <Select options={options} onChange={picked} label="Kind" />,
    open: () => {
      const trigger = screen.getByRole('button', { name: 'Select…' });
      fireEvent.click(trigger);
      return trigger;
    },
  },
  {
    name: 'ListBox',
    mount: (picked) => (
      <ListBox options={options} onChange={picked as (v: string & string[]) => void} label="Kind" />
    ),
    open: () => screen.getByRole('listbox'),
  },
  {
    name: 'Combobox',
    mount: (picked) => <Combobox options={options} onChange={picked} label="Kind" />,
    open: () => {
      const input = screen.getByRole('combobox');
      fireEvent.focus(input);
      return input as HTMLElement;
    },
  },
  {
    name: 'DropdownMenu',
    mount: (picked) => (
      <DropdownMenu label="Actions" items={LABELS.map((l) => ({ label: l, onSelect: () => picked(l) }))} />
    ),
    open: () => {
      const trigger = screen.getByRole('button', { name: 'Actions' });
      fireEvent.click(trigger);
      return trigger;
    },
  },
  {
    name: 'Command',
    mount: (picked) => (
      <Command open items={LABELS.map((l) => ({ label: l, onSelect: () => picked(l) }))} />
    ),
    open: () => screen.getByPlaceholderText('Type a command…'),
  },
];

/** Mount a widget, open it, press the keys in order, and report what it picked. */
function drive(widget: Widget, keys: string[]): string | undefined {
  let picked: string | undefined;
  render(() => widget.mount((label) => { picked = label; }));
  const target = widget.open();
  for (const key of keys) fireEvent.keyDown(target, { key });
  return picked;
}

for (const widget of widgets) {
  describe(`the roving index in a ${widget.name}`, () => {
    it('moves forward one item for each ArrowDown', () => {
      expect(drive(widget, ['Home', 'ArrowDown', 'ArrowDown', 'Enter'])).toBe('Gamma');
    });

    it('moves back one item for each ArrowUp', () => {
      expect(drive(widget, ['End', 'ArrowUp', 'Enter'])).toBe('Beta');
    });

    it('goes to the first item on Home', () => {
      expect(drive(widget, ['End', 'Home', 'Enter'])).toBe('Alpha');
    });

    it('goes to the last item on End', () => {
      expect(drive(widget, ['End', 'Enter'])).toBe('Gamma');
    });

    it('wraps to the first item when ArrowDown passes the end', () => {
      expect(drive(widget, ['End', 'ArrowDown', 'Enter'])).toBe('Alpha');
    });

    it('wraps to the last item when ArrowUp passes the start', () => {
      expect(drive(widget, ['Home', 'ArrowUp', 'Enter'])).toBe('Gamma');
    });
  });
}

describe('items the roving index cannot land on', () => {
  const disabled = [
    { value: 'Alpha', label: 'Alpha' },
    { value: 'Beta', label: 'Beta', disabled: true },
    { value: 'Gamma', label: 'Gamma' },
  ];

  it('skips a disabled option', () => {
    let picked: string | undefined;
    render(() => <ListBox options={disabled} onChange={((v: string) => { picked = v; }) as (v: string & string[]) => void} label="Kind" />);
    const listbox = screen.getByRole('listbox');
    fireEvent.keyDown(listbox, { key: 'Home' });
    fireEvent.keyDown(listbox, { key: 'ArrowDown' });
    fireEvent.keyDown(listbox, { key: 'Enter' });
    expect(picked).toBe('Gamma');
  });

  it('skips a separator in a menu', () => {
    let picked: string | undefined;
    render(() => (
      <DropdownMenu label="Actions" items={[
        { label: 'Alpha', onSelect: () => { picked = 'Alpha'; } },
        { separator: true },
        { label: 'Gamma', onSelect: () => { picked = 'Gamma'; } },
      ]} />
    ));
    const trigger = screen.getByRole('button', { name: 'Actions' });
    fireEvent.click(trigger);
    fireEvent.keyDown(trigger, { key: 'Home' });
    fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    fireEvent.keyDown(trigger, { key: 'Enter' });
    expect(picked).toBe('Gamma');
  });

  it('picks nothing when every option is disabled', () => {
    let picked: string | undefined;
    const all = LABELS.map((label) => ({ value: label, label, disabled: true }));
    render(() => <ListBox options={all} onChange={((v: string) => { picked = v; }) as (v: string & string[]) => void} label="Kind" />);
    const listbox = screen.getByRole('listbox');
    for (const key of ['End', 'ArrowDown', 'Home', 'ArrowUp', 'Enter']) {
      fireEvent.keyDown(listbox, { key });
    }
    expect(picked).toBeUndefined();
  });

  it('keeps its place when the item list changes under it', () => {
    /* A command palette whose items arrive late, or change as the app works,
       must not throw the keyboard user back to the top of the list. */
    let picked: string | undefined;
    const [items, setItems] = createSignal(
      LABELS.map((l) => ({ label: l, onSelect: () => { picked = l; } })),
    );
    render(() => <Command open items={items()} />);
    const input = screen.getByPlaceholderText('Type a command…');
    fireEvent.keyDown(input, { key: 'End' });
    setItems([...items()]);
    fireEvent.keyDown(input, { key: 'Enter' });
    expect(picked).toBe('Gamma');
  });

  it('moves nowhere in an empty list', () => {
    let picked: string | undefined;
    render(() => <Combobox options={options} onChange={(v) => { picked = v; }} label="Kind" />);
    const input = screen.getByRole('combobox');
    fireEvent.focus(input);
    fireEvent.input(input, { target: { value: 'no such option' } });
    for (const key of ['ArrowDown', 'End', 'Home', 'Enter']) fireEvent.keyDown(input, { key });
    expect(picked).toBeUndefined();
  });
});
