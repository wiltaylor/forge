/* The overlay module's interaction model, tested through the components that
   adapt it: what is in the document after a real pointer or key event. */
import { describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import { fireEvent, render, screen } from '@solidjs/testing-library';
import { Command, ContextMenu, DropdownMenu, Modal, Popover, Sheet } from '../src/overlays';
import { Combobox, Select } from '../src/forms';
import { DatePicker } from '../src/date';

const openPopover = () => fireEvent.click(screen.getByRole('button', { name: 'Filters' }));

/* Every anchored surface in the kit: how to mount it, how its own trigger opens
   it, and one thing inside it that says whether it is open. */
const anchored = [
  {
    name: 'Popover',
    mount: () => <Popover label="Filters"><p>Alpha</p></Popover>,
    open: openPopover,
    item: () => screen.queryByText('Alpha'),
  },
  {
    name: 'DropdownMenu',
    mount: () => <DropdownMenu label="Filters" items={[{ label: 'Alpha' }]} />,
    open: openPopover,
    item: () => screen.queryByRole('menuitem', { name: 'Alpha' }),
  },
  {
    name: 'Select',
    mount: () => <Select options={[{ value: 'a', label: 'Alpha' }]} label="Kind" />,
    open: () => fireEvent.click(screen.getByRole('button', { name: 'Select…' })),
    item: () => screen.queryByRole('option', { name: 'Alpha' }),
  },
  {
    name: 'Combobox',
    mount: () => <Combobox options={[{ value: 'a', label: 'Alpha' }]} label="Kind" />,
    open: () => fireEvent.focus(screen.getByRole('combobox')),
    item: () => screen.queryByRole('option', { name: 'Alpha' }),
  },
  {
    name: 'DatePicker',
    mount: () => <DatePicker value="2026-08-01" label="Day" />,
    open: () => fireEvent.click(screen.getByRole('button', { name: /2026-08-01/ })),
    item: () => screen.queryByRole('button', { name: 'Previous month' }),
  },
  {
    name: 'ContextMenu',
    mount: () => <ContextMenu items={[{ label: 'Alpha' }]}><span>Surface</span></ContextMenu>,
    open: () => fireEvent.contextMenu(screen.getByText('Surface')),
    item: () => screen.queryByRole('menuitem', { name: 'Alpha' }),
  },
];

describe('dismissal on outside interaction', () => {
  for (const overlay of anchored) {
    it(`closes a ${overlay.name} when the pointer lands outside it`, () => {
      render(() => (
        <div>
          <button type="button">Elsewhere</button>
          {overlay.mount()}
        </div>
      ));
      overlay.open();
      expect(overlay.item()).toBeTruthy();

      fireEvent.pointerDown(screen.getByRole('button', { name: 'Elsewhere' }));

      expect(overlay.item()).toBeNull();
    });

    it(`leaves a ${overlay.name} open when the pointer lands inside it`, () => {
      render(() => overlay.mount());
      overlay.open();

      fireEvent.pointerDown(overlay.item()!);

      expect(overlay.item()).toBeTruthy();
    });
  }

  /* Modal, Sheet and Command each carried their own dismiss-on-backdrop before
     the overlay module; one rule now covers all three. */
  const backdrop = [
    {
      name: 'Modal',
      mount: (open: () => boolean, close: () => void) => (
        <Modal open={open()} onClose={close} title="Settings">
          <p>Dialog body</p>
        </Modal>
      ),
    },
    {
      name: 'Sheet',
      mount: (open: () => boolean, close: () => void) => (
        <Sheet open={open()} onClose={close} title="Details">
          <p>Dialog body</p>
        </Sheet>
      ),
    },
    {
      name: 'Command',
      mount: (open: () => boolean, close: () => void) => (
        <Command open={open()} onClose={close} items={[{ label: 'Dialog body' }]} />
      ),
    },
  ];

  for (const surface of backdrop) {
    it(`closes a ${surface.name} when the pointer lands outside the panel`, () => {
      const [open, setOpen] = createSignal(true);
      render(() => surface.mount(open, () => setOpen(false)));
      /* The dim area around the panel is what a user aims at, and it has no
         role of its own to query by. */
      const dim = screen.getByRole('dialog').parentElement!;

      fireEvent.pointerDown(dim);

      expect(screen.queryByRole('dialog')).toBeNull();
    });

    it(`leaves a ${surface.name} open when the pointer lands inside the panel`, () => {
      const [open, setOpen] = createSignal(true);
      render(() => surface.mount(open, () => setOpen(false)));

      fireEvent.pointerDown(screen.getByText('Dialog body'));

      expect(screen.getByRole('dialog')).toBeTruthy();
    });

    /* A toast portals above the backdrop, so a pointer reaching it never went
       through the backdrop and is not this surface's outside. */
    it(`leaves a ${surface.name} open when the pointer lands above the backdrop`, () => {
      const [open, setOpen] = createSignal(true);
      render(() => (
        <>
          {surface.mount(open, () => setOpen(false))}
          <button type="button">Undo</button>
        </>
      ));

      fireEvent.pointerDown(screen.getByRole('button', { name: 'Undo' }));

      expect(screen.getByRole('dialog')).toBeTruthy();
    });
  }
});

describe('dismissal on Escape', () => {
  for (const overlay of anchored) {
    it(`closes a ${overlay.name} while focus is elsewhere`, () => {
      render(() => overlay.mount());
      overlay.open();
      expect(overlay.item()).toBeTruthy();

      fireEvent.keyDown(document.body, { key: 'Escape' });

      expect(overlay.item()).toBeNull();
    });
  }

  /* The Select handled Escape on its trigger button, so it stayed open once
     anything else took focus. It listens where its siblings do now. */
  it('closes a Select regardless of what has focus', () => {
    render(() => (
      <div>
        <input aria-label="Note" />
        <Select options={[{ value: 'a', label: 'Alpha' }]} label="Kind" />
      </div>
    ));
    fireEvent.click(screen.getByRole('button', { name: 'Select…' }));
    expect(screen.getByRole('listbox')).toBeTruthy();

    const note = screen.getByRole<HTMLInputElement>('textbox', { name: 'Note' });
    note.focus();
    fireEvent.keyDown(note, { key: 'Escape' });

    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('closes a Modal', () => {
    const [open, setOpen] = createSignal(true);
    render(() => (
      <Modal open={open()} onClose={() => setOpen(false)} title="Settings">
        <p>Dialog body</p>
      </Modal>
    ));

    fireEvent.keyDown(document.body, { key: 'Escape' });

    expect(screen.queryByRole('dialog')).toBeNull();
  });
});

describe('Escape precedence between nested overlays', () => {
  const nested = () => {
    const [open, setOpen] = createSignal(true);
    render(() => (
      <Modal open={open()} onClose={() => setOpen(false)} title="Settings">
        <Popover label="Filters">
          <p>Panel body</p>
        </Popover>
      </Modal>
    ));
    openPopover();
  };

  it('reaches the innermost overlay and leaves the one around it open', () => {
    nested();
    expect(screen.getByText('Panel body')).toBeTruthy();

    fireEvent.keyDown(document.body, { key: 'Escape' });

    expect(screen.queryByText('Panel body')).toBeNull();
    expect(screen.getByRole('dialog')).toBeTruthy();
  });

  it('reaches the outer overlay on the next Escape', () => {
    nested();

    fireEvent.keyDown(document.body, { key: 'Escape' });
    fireEvent.keyDown(document.body, { key: 'Escape' });

    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('leaves both open when the pointer lands inside the innermost', () => {
    nested();

    fireEvent.pointerDown(screen.getByText('Panel body'));

    expect(screen.getByText('Panel body')).toBeTruthy();
    expect(screen.getByRole('dialog')).toBeTruthy();
  });

  /* Both of these portal through the mount seam, so neither contains the
     other: precedence is the order they opened in, not where they sit. */
  it('reaches the innermost when both overlays are portalled', () => {
    const [sheet, setSheet] = createSignal(true);
    const [command, setCommand] = createSignal(false);
    render(() => (
      <>
        <Sheet open={sheet()} onClose={() => setSheet(false)} title="Details">
          <p>Sheet body</p>
        </Sheet>
        <Command open={command()} onClose={() => setCommand(false)} items={[]} />
      </>
    ));
    setCommand(true);
    expect(screen.getByRole('dialog', { name: 'Command palette' })).toBeTruthy();

    fireEvent.keyDown(document.body, { key: 'Escape' });

    expect(screen.queryByRole('dialog', { name: 'Command palette' })).toBeNull();
    expect(screen.getByRole('dialog', { name: 'Details' })).toBeTruthy();
  });
});
